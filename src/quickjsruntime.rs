// store in thread_local

use crate::eserror::EsError;
use crate::esruntime::EsRuntime;
use crate::esscript::EsScript;
use crate::quickjs_utils::{gc, modules, promises};
use crate::quickjscontext::QuickJsContext;
use libquickjs_sys as q;
use std::cell::RefCell;
use std::collections::HashMap;
use std::ffi::CString;
use std::panic;
use std::sync::{Arc, Weak};

pub type ModuleScriptLoader =
    dyn Fn(&QuickJsContext, &str, &str) -> Option<EsScript> + Send + Sync + 'static;

thread_local! {
   /// the thread-local QuickJsRuntime
   /// this only exists for the worker thread of the EsEventQueue
   pub(crate) static QJS_RT: RefCell<QuickJsRuntime> = RefCell::new(QuickJsRuntime::new());

}

pub struct QuickJsRuntime {
    pub(crate) runtime: *mut q::JSRuntime,
    contexts: HashMap<String, QuickJsContext>,
    es_rt_ref: Option<Weak<EsRuntime>>,
    id: String,
    context_init_hooks:
        RefCell<Vec<Box<dyn Fn(&QuickJsRuntime, &QuickJsContext) -> Result<(), EsError>>>>,
    pub(crate) module_script_loader: Option<Box<ModuleScriptLoader>>,
}

impl QuickJsRuntime {
    pub fn add_context_init_hook<H>(&self, hook: H) -> Result<(), EsError>
    where
        H: Fn(&QuickJsRuntime, &QuickJsContext) -> Result<(), EsError> + 'static,
    {
        // todo, for each context run hook

        for ctx in self.contexts.values() {
            hook(self, ctx)?;
        }

        let hooks = &mut *self.context_init_hooks.borrow_mut();
        hooks.push(Box::new(hook));
        Ok(())
    }
    pub fn create_context(&mut self, id: &str) -> Result<&QuickJsContext, EsError> {
        assert!(!self.has_context(id));
        {
            let ctx = QuickJsContext::new(id.to_string(), self);

            let hooks = &*self.context_init_hooks.borrow();
            for hook in hooks {
                hook(self, &ctx)?;
            }

            self.contexts.insert(id.to_string(), ctx);
        }

        Ok(self.get_context(id))
    }
    pub fn drop_context(&mut self, id: &str) {
        assert!(self.has_context(id));
        self.gc();
        self.contexts.remove(id);
    }
    pub fn get_context(&self, id: &str) -> &QuickJsContext {
        self.contexts.get(id).expect("no such context")
    }
    pub fn opt_context(&self, id: &str) -> Option<&QuickJsContext> {
        self.contexts.get(id)
    }
    pub fn has_context(&self, id: &str) -> bool {
        self.contexts.contains_key(id)
    }
    pub(crate) fn init_rt_ref(&mut self, rt_ref: Arc<EsRuntime>) {
        self.es_rt_ref = Some(Arc::downgrade(&rt_ref));
    }
    pub fn get_quickjs_context(&self, context: *mut q::JSContext) -> &QuickJsContext {
        let id = QuickJsContext::get_id(context);
        self.get_context(id)
    }
    pub fn get_rt_ref(&self) -> Option<Arc<EsRuntime>> {
        if let Some(rt_ref) = &self.es_rt_ref {
            rt_ref.upgrade()
        } else {
            None
        }
    }
    fn new() -> Self {
        log::trace!("creating new QuickJsRuntime");
        let runtime = unsafe { q::JS_NewRuntime() };
        if runtime.is_null() {
            panic!("RuntimeCreationFailed");
        }

        // Configure memory limit if specified.
        //let memory_limit = None;
        //if let Some(limit) = memory_limit {
        //  unsafe {
        //q::JS_SetMemoryLimit(runtime, limit as _);
        //}
        //}

        let id = format!("q_{}", thread_id::get());

        let mut q_rt = Self {
            runtime,
            contexts: Default::default(),

            es_rt_ref: None,
            id,
            context_init_hooks: RefCell::new(vec![]),
            module_script_loader: None,
        };

        modules::set_module_loader(&q_rt);
        promises::init_promise_rejection_tracker(&q_rt);

        q_rt.create_context("__main__")
            .ok()
            .expect("could not init main context");

        q_rt
    }

    pub fn get_main_context(&self) -> &QuickJsContext {
        // todo store this somewhere so we don't need a lookup in the map every time
        self.get_context("__main__")
    }

    /// run the garbage collector
    pub fn gc(&self) {
        gc(self);
    }

    pub fn do_with<C, R>(task: C) -> R
    where
        C: FnOnce(&QuickJsRuntime) -> R,
    {
        QJS_RT.with(|qjs_rc| {
            let qjs_rt = &*qjs_rc.borrow();
            task(qjs_rt)
        })
    }

    pub fn do_with_mut<C, R>(task: C) -> R
    where
        C: FnOnce(&mut QuickJsRuntime) -> R,
    {
        QJS_RT.with(|qjs_rc| {
            let qjs_rt = &mut *qjs_rc.borrow_mut();
            task(qjs_rt)
        })
    }

    pub fn has_pending_jobs(&self) -> bool {
        let flag = unsafe { q::JS_IsJobPending(self.runtime) };
        flag > 0
    }

    pub fn run_pending_job(&self) -> Result<(), EsError> {
        let mut ctx: *mut q::JSContext = std::ptr::null_mut();
        let flag = unsafe {
            // ctx is a return arg here
            q::JS_ExecutePendingJob(self.runtime, &mut ctx)
        };
        if flag < 0 {
            let e = QuickJsContext::get_exception(ctx)
                .unwrap_or_else(|| EsError::new_str("Unknown exception while running pending job"));
            return Err(e);
        }
        Ok(())
    }

    pub fn get_id(&self) -> &str {
        self.id.as_str()
    }
}

impl Drop for QuickJsRuntime {
    fn drop(&mut self) {
        log::trace!("before JS_FreeRuntime");

        self.gc();

        unsafe { q::JS_FreeRuntime(self.runtime) };
        log::trace!("after drop QuickJsRuntime");
    }
}

/// Helper for creating CStrings.
pub(crate) fn make_cstring(value: &str) -> Result<CString, EsError> {
    let res = CString::new(value);
    match res {
        Ok(val) => Ok(val),
        Err(_) => Err(EsError::new_string(format!(
            "could not create cstring from {}",
            value
        ))),
    }
}

#[cfg(test)]
pub mod tests {
    use crate::esscript::EsScript;
    use crate::quickjsruntime::QuickJsRuntime;

    #[test]
    fn test_rt() {
        log::info!("> test_rt");

        let rt = QuickJsRuntime::new();
        rt.get_main_context()
            .eval(EsScript::new("test.es", "1+1;"))
            .ok()
            .expect("could not eval");

        log::info!("< test_rt");
    }
}
