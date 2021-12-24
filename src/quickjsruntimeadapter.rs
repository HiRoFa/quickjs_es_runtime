// store in thread_local

use crate::facades::{QuickJsRuntimeFacade, QuickjsRuntimeFacadeInner};
use crate::quickjs_utils::compile::from_bytecode;
use crate::quickjs_utils::modules::{
    add_module_export, compile_module, get_module_def, get_module_name, new_module,
    set_module_export,
};
use crate::quickjs_utils::{gc, interrupthandler, modules, promises};
use crate::quickjsrealmadapter::QuickJsRealmAdapter;
use hirofa_utils::js_utils::adapters::JsRuntimeAdapter;
use hirofa_utils::js_utils::modules::{
    CompiledModuleLoader, NativeModuleLoader, ScriptModuleLoader,
};
use hirofa_utils::js_utils::JsError;
use hirofa_utils::js_utils::Script;
use hirofa_utils::js_utils::ScriptPreProcessor;
use libquickjs_sys as q;
use std::cell::RefCell;
use std::collections::HashMap;
use std::ffi::CString;
use std::os::raw::c_int;
use std::panic;
use std::sync::{Arc, Weak};

/// this is the internal abstract loader which is used to actually load the modules
pub trait ModuleLoader {
    /// the normalize methods is used to translate a possible relative path to an absolute path of a module
    /// it doubles as a method to see IF a module can actually be loaded by a module loader (return None if the module can not be found)
    fn normalize_path(
        &self,
        q_ctx: &QuickJsRealmAdapter,
        ref_path: &str,
        path: &str,
    ) -> Option<String>;
    /// load the Module
    fn load_module(
        &self,
        q_ctx: &QuickJsRealmAdapter,
        absolute_path: &str,
    ) -> Result<*mut q::JSModuleDef, JsError>;
    /// has module is used to check if a loader can provide a certain module, this is currently used to check which loader should init a native module
    fn has_module(&self, q_ctx: &QuickJsRealmAdapter, absolute_path: &str) -> bool;
    /// init a module, currently used to init native modules
    /// # Safety
    /// be safe with the moduledef ptr
    unsafe fn init_module(
        &self,
        q_ctx: &QuickJsRealmAdapter,
        module: *mut q::JSModuleDef,
    ) -> Result<(), JsError>;
}

// these are the external (util) loaders (todo move these to esruntime?)

pub struct CompiledModuleLoaderAdapter {
    inner: Box<dyn CompiledModuleLoader<QuickJsRealmAdapter>>,
}

impl CompiledModuleLoaderAdapter {
    pub fn new(loader: Box<dyn CompiledModuleLoader<QuickJsRealmAdapter>>) -> Self {
        Self { inner: loader }
    }
}

pub struct ScriptModuleLoaderAdapter {
    inner: Box<dyn ScriptModuleLoader<QuickJsRealmAdapter>>,
}

impl ScriptModuleLoaderAdapter {
    pub fn new(loader: Box<dyn ScriptModuleLoader<QuickJsRealmAdapter>>) -> Self {
        Self { inner: loader }
    }
}

impl ModuleLoader for CompiledModuleLoaderAdapter {
    fn normalize_path(
        &self,
        q_ctx: &QuickJsRealmAdapter,
        ref_path: &str,
        path: &str,
    ) -> Option<String> {
        self.inner.normalize_path(q_ctx, ref_path, path)
    }

    fn load_module(
        &self,
        q_ctx: &QuickJsRealmAdapter,
        absolute_path: &str,
    ) -> Result<*mut q::JSModuleDef, JsError> {
        let bytes = self.inner.load_module(q_ctx, absolute_path);

        let compiled_module = unsafe { from_bytecode(q_ctx.context, &bytes)? };
        Ok(get_module_def(&compiled_module))
    }

    fn has_module(&self, q_ctx: &QuickJsRealmAdapter, absolute_path: &str) -> bool {
        self.normalize_path(q_ctx, absolute_path, absolute_path)
            .is_some()
    }

    unsafe fn init_module(
        &self,
        _q_ctx: &QuickJsRealmAdapter,
        _module: *mut q::JSModuleDef,
    ) -> Result<(), JsError> {
        Ok(())
    }
}

impl ModuleLoader for ScriptModuleLoaderAdapter {
    fn normalize_path(
        &self,
        realm: &QuickJsRealmAdapter,
        ref_path: &str,
        path: &str,
    ) -> Option<String> {
        self.inner.normalize_path(realm, ref_path, path)
    }

    fn load_module(
        &self,
        realm: &QuickJsRealmAdapter,
        absolute_path: &str,
    ) -> Result<*mut q::JSModuleDef, JsError> {
        let code = self.inner.load_module(realm, absolute_path);

        let mut script = Script::new(absolute_path, code.as_str());
        script = QuickJsRuntimeAdapter::pre_process(script)?;

        let compiled_module = unsafe { compile_module(realm.context, script)? };
        Ok(get_module_def(&compiled_module))
    }

    fn has_module(&self, q_ctx: &QuickJsRealmAdapter, absolute_path: &str) -> bool {
        self.normalize_path(q_ctx, absolute_path, absolute_path)
            .is_some()
    }

    unsafe fn init_module(
        &self,
        _q_ctx: &QuickJsRealmAdapter,
        _module: *mut q::JSModuleDef,
    ) -> Result<(), JsError> {
        Ok(())
    }
}

pub struct NativeModuleLoaderAdapter {
    inner: Box<dyn NativeModuleLoader<QuickJsRealmAdapter>>,
}

impl NativeModuleLoaderAdapter {
    pub fn new(loader: Box<dyn NativeModuleLoader<QuickJsRealmAdapter>>) -> Self {
        Self { inner: loader }
    }
}

impl ModuleLoader for NativeModuleLoaderAdapter {
    fn normalize_path(
        &self,
        q_ctx: &QuickJsRealmAdapter,
        _ref_path: &str,
        path: &str,
    ) -> Option<String> {
        if self.inner.has_module(q_ctx, path) {
            Some(path.to_string())
        } else {
            None
        }
    }

    fn load_module(
        &self,
        q_ctx: &QuickJsRealmAdapter,
        absolute_path: &str,
    ) -> Result<*mut q::JSModuleDef, JsError> {
        // create module
        let module = unsafe { new_module(q_ctx.context, absolute_path, Some(native_module_init))? };

        for name in self.inner.get_module_export_names(q_ctx, absolute_path) {
            unsafe { add_module_export(q_ctx.context, module, name)? }
        }

        //std::ptr::null_mut()
        Ok(module)
    }

    fn has_module(&self, q_ctx: &QuickJsRealmAdapter, absolute_path: &str) -> bool {
        self.inner.has_module(q_ctx, absolute_path)
    }

    unsafe fn init_module(
        &self,
        q_ctx: &QuickJsRealmAdapter,
        module: *mut q::JSModuleDef,
    ) -> Result<(), JsError> {
        let module_name = get_module_name(q_ctx.context, module)?;

        for (name, val) in self.inner.get_module_exports(q_ctx, module_name.as_str()) {
            set_module_export(q_ctx.context, module, name, val)?;
        }
        Ok(())
    }
}

unsafe extern "C" fn native_module_init(
    ctx: *mut q::JSContext,
    module: *mut q::JSModuleDef,
) -> c_int {
    let module_name = get_module_name(ctx, module)
        .ok()
        .expect("could not get name");
    log::trace!("native_module_init: {}", module_name);

    QuickJsRuntimeAdapter::do_with(|q_js_rt| {
        QuickJsRealmAdapter::with_context(ctx, |q_ctx| {
            if let Some(res) = q_js_rt.with_all_module_loaders(|module_loader| {
                if module_loader.has_module(q_ctx, module_name.as_str()) {
                    match module_loader.init_module(q_ctx, module) {
                        Ok(_) => {
                            Some(0) // ok
                        }
                        Err(e) => {
                            q_ctx.report_ex(
                                format!(
                                    "Failed to init native module: {} caused by {}",
                                    module_name, e
                                )
                                .as_str(),
                            );
                            Some(1)
                        }
                    }
                } else {
                    None
                }
            }) {
                res
            } else {
                0
            }
        })
    })
}

thread_local! {
   /// the thread-local QuickJsRuntime
   /// this only exists for the worker thread of the EsEventQueue
   /// todo move rt init to toplevel stackframe (out of lazy init)
   /// so the thread_local should be a refcel containing a null reF? or a None
   pub(crate) static QJS_RT: RefCell<Option<QuickJsRuntimeAdapter >> = {
       RefCell::new(None)
   };

}

pub type ContextInitHooks =
    Vec<Box<dyn Fn(&QuickJsRuntimeAdapter, &QuickJsRealmAdapter) -> Result<(), JsError>>>;

pub struct QuickJsRuntimeAdapter {
    pub(crate) runtime: *mut q::JSRuntime,
    pub(crate) contexts: HashMap<String, QuickJsRealmAdapter>,
    rti_ref: Option<Weak<QuickjsRuntimeFacadeInner>>,
    id: String,
    pub(crate) context_init_hooks: RefCell<ContextInitHooks>,
    script_module_loaders: Vec<ScriptModuleLoaderAdapter>,
    native_module_loaders: Vec<NativeModuleLoaderAdapter>,
    compiled_module_loaders: Vec<CompiledModuleLoaderAdapter>,
    pub(crate) script_pre_processors: Vec<Box<dyn ScriptPreProcessor + Send>>,
    pub(crate) interrupt_handler: Option<Box<dyn Fn(&QuickJsRuntimeAdapter) -> bool>>,
}

thread_local! {
    static NESTED: RefCell<bool> = RefCell::new(false);
}

impl QuickJsRuntimeAdapter {
    pub(crate) fn init_rt_for_current_thread(rt: QuickJsRuntimeAdapter) {
        QJS_RT.with(|rc| {
            let opt = &mut *rc.borrow_mut();
            opt.replace(rt);
        })
    }

    pub(crate) fn pre_process(mut script: Script) -> Result<Script, JsError> {
        Self::do_with(|q_js_rt| {
            for pp in &q_js_rt.script_pre_processors {
                pp.process(&mut script)?;
            }
            Ok(script)
        })
    }

    pub fn add_context_init_hook<H>(&self, hook: H) -> Result<(), JsError>
    where
        H: Fn(&QuickJsRuntimeAdapter, &QuickJsRealmAdapter) -> Result<(), JsError> + 'static,
    {
        let i = {
            let hooks = &mut *self.context_init_hooks.borrow_mut();
            hooks.push(Box::new(hook));
            hooks.len() - 1
        };

        let hooks = &*self.context_init_hooks.borrow();
        let hook = hooks.get(i).expect("invalid state");
        for ctx in self.contexts.values() {
            if let Err(e) = hook(self, ctx) {
                panic!("hook failed {}", e);
            }
        }

        Ok(())
    }
    // todo, this needs to be static, create a context, then borrowmut and add it (do not borrow mut while instantiating context)
    // so actually needs to be called in a plain job to inner.TaskManager and not by add_to_esEventquueue
    // EsRuntime should have a util to do that
    // EsRuntime should have extra methods like eval_sync_ctx(ctx: &str, script: &Script) etc
    pub fn create_context(id: &str) -> Result<(), JsError> {
        let ctx = Self::do_with(|q_js_rt| {
            assert!(!q_js_rt.has_context(id));
            QuickJsRealmAdapter::new(id.to_string(), q_js_rt)
        });

        QuickJsRuntimeAdapter::do_with_mut(|q_js_rt| {
            q_js_rt.contexts.insert(id.to_string(), ctx);
        });

        Self::do_with(|q_js_rt| {
            let ctx = q_js_rt.get_context(id);
            let hooks = &*q_js_rt.context_init_hooks.borrow();
            for hook in hooks {
                hook(q_js_rt, ctx)?;
            }
            Ok(())
        })
    }
    pub fn remove_context(id: &str) {
        log::debug!("QuickJsRuntime::drop_context: {}", id);

        QuickJsRuntimeAdapter::do_with(|rt| {
            let q_ctx = rt.get_context(id);
            log::trace!("QuickJsRuntime::q_ctx.free: {}", id);
            q_ctx.free();
            log::trace!("after QuickJsRuntime::q_ctx.free: {}", id);
            rt.gc();
        });

        let ctx = QuickJsRuntimeAdapter::do_with_mut(|m_rt| {
            m_rt.contexts.remove(id).expect("no such context")
        });

        drop(ctx);
    }
    pub(crate) fn get_context_ids() -> Vec<String> {
        QuickJsRuntimeAdapter::do_with(|q_js_rt| {
            q_js_rt.contexts.iter().map(|c| c.0.clone()).collect()
        })
    }
    pub fn get_context(&self, id: &str) -> &QuickJsRealmAdapter {
        self.contexts.get(id).expect("no such context")
    }
    pub fn opt_context(&self, id: &str) -> Option<&QuickJsRealmAdapter> {
        self.contexts.get(id)
    }
    pub fn has_context(&self, id: &str) -> bool {
        self.contexts.contains_key(id)
    }
    pub(crate) fn init_rti_ref(&mut self, el_ref: Weak<QuickjsRuntimeFacadeInner>) {
        self.rti_ref = Some(el_ref);
    }
    /// # Safety
    /// When passing a context pointer please make sure the corresponding QuickJsContext is still valid
    pub unsafe fn get_quickjs_context(&self, context: *mut q::JSContext) -> &QuickJsRealmAdapter {
        let id = QuickJsRealmAdapter::get_id(context);
        self.get_context(id)
    }

    pub fn get_rti_ref(&self) -> Option<Arc<QuickjsRuntimeFacadeInner>> {
        if let Some(rt_ref) = &self.rti_ref {
            rt_ref.upgrade()
        } else {
            None
        }
    }

    pub(crate) fn new(runtime: *mut q::JSRuntime) -> Self {
        log::trace!("creating new QuickJsRuntime");

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

            rti_ref: None,
            id,
            context_init_hooks: RefCell::new(vec![]),
            script_module_loaders: vec![],
            native_module_loaders: vec![],
            compiled_module_loaders: vec![],
            script_pre_processors: vec![],
            interrupt_handler: None,
        };

        modules::set_module_loader(&q_rt);
        promises::init_promise_rejection_tracker(&q_rt);

        let main_ctx = QuickJsRealmAdapter::new("__main__".to_string(), &q_rt);
        q_rt.contexts.insert("__main__".to_string(), main_ctx);

        q_rt
    }

    pub fn set_interrupt_handler<I: Fn(&QuickJsRuntimeAdapter) -> bool + 'static>(
        &mut self,
        interrupt_handler: I,
    ) -> &mut Self {
        self.interrupt_handler = Some(Box::new(interrupt_handler));
        interrupthandler::init(self);
        self
    }

    pub fn add_script_module_loader(&mut self, sml: ScriptModuleLoaderAdapter) {
        self.script_module_loaders.push(sml);
    }

    pub fn add_compiled_module_loader(&mut self, cml: CompiledModuleLoaderAdapter) {
        self.compiled_module_loaders.push(cml);
    }

    pub fn add_native_module_loader(&mut self, nml: NativeModuleLoaderAdapter) {
        self.native_module_loaders.push(nml);
    }

    pub fn get_main_context(&self) -> &QuickJsRealmAdapter {
        // todo store this somewhere so we don't need a lookup in the map every time
        self.get_context("__main__")
    }

    pub fn with_all_module_loaders<C, R>(&self, consumer: C) -> Option<R>
    where
        C: Fn(&dyn ModuleLoader) -> Option<R>,
    {
        for loader in &self.compiled_module_loaders {
            let res = consumer(loader);
            if res.is_some() {
                return res;
            }
        }
        for loader in &self.native_module_loaders {
            let res = consumer(loader);
            if res.is_some() {
                return res;
            }
        }
        for loader in &self.script_module_loaders {
            let res = consumer(loader);
            if res.is_some() {
                return res;
            }
        }
        None
    }

    /// run the garbage collector
    pub fn gc(&self) {
        gc(self);
    }

    pub fn do_with<C, R>(task: C) -> R
    where
        C: FnOnce(&QuickJsRuntimeAdapter) -> R,
    {
        let most_outer = NESTED.with(|rc| {
            if *rc.borrow() {
                false
            } else {
                *rc.borrow_mut() = true;
                true
            }
        });

        let res = QJS_RT.with(|qjs_rc| {
            let qjs_rt_opt = &*qjs_rc.borrow();
            let q_js_rt = qjs_rt_opt
                .as_ref()
                .expect("runtime was not yet initialized for this thread");
            if most_outer {
                unsafe { libquickjs_sys::JS_UpdateStackTop(q_js_rt.runtime) };
            }
            task(q_js_rt)
        });

        if most_outer {
            NESTED.with(|rc| {
                *rc.borrow_mut() = false;
            });
        }

        res
    }

    pub fn do_with_mut<C, R>(task: C) -> R
    where
        C: FnOnce(&mut QuickJsRuntimeAdapter) -> R,
    {
        let most_outer = NESTED.with(|rc| {
            if *rc.borrow() {
                false
            } else {
                *rc.borrow_mut() = true;
                true
            }
        });

        let res = QJS_RT.with(|qjs_rc| {
            let qjs_rt_opt = &mut *qjs_rc.borrow_mut();
            let qjs_rt = qjs_rt_opt
                .as_mut()
                .expect("runtime was not yet initialized for this thread");
            if most_outer {
                unsafe { libquickjs_sys::JS_UpdateStackTop(qjs_rt.runtime) };
            }
            task(qjs_rt)
        });

        if most_outer {
            NESTED.with(|rc| {
                *rc.borrow_mut() = false;
            });
        }

        res
    }

    /// run pending jobs if avail
    /// # todo
    /// move this to a quickjs_utils::pending_jobs so it can be used without doing QuickjsRuntime.do_with()
    pub fn run_pending_jobs_if_any(&self) {
        log::trace!("quick_js_rt.run_pending_jobs_if_any");
        while self.has_pending_jobs() {
            log::trace!("quick_js_rt.has_pending_jobs!");
            let res = self.run_pending_job();
            match res {
                Ok(_) => {
                    log::trace!("run_pending_job OK!");
                }
                Err(e) => {
                    log::error!("run_pending_job failed: {}", e);
                }
            }
        }
    }

    pub fn has_pending_jobs(&self) -> bool {
        let flag = unsafe { q::JS_IsJobPending(self.runtime) };
        flag > 0
    }

    pub fn run_pending_job(&self) -> Result<(), JsError> {
        let mut ctx: *mut q::JSContext = std::ptr::null_mut();
        let flag = unsafe {
            // ctx is a return arg here
            q::JS_ExecutePendingJob(self.runtime, &mut ctx)
        };
        if flag < 0 {
            let e = unsafe { QuickJsRealmAdapter::get_exception(ctx) }
                .unwrap_or_else(|| JsError::new_str("Unknown exception while running pending job"));
            return Err(e);
        }
        Ok(())
    }

    pub fn get_id(&self) -> &str {
        self.id.as_str()
    }

    /// this method tries to load a module script using the runtimes script_module loaders
    pub fn load_module_script_opt(&self, ref_path: &str, path: &str) -> Option<Script> {
        let realm = self.js_get_main_realm();
        for loader in &self.script_module_loaders {
            let i = &loader.inner;
            if let Some(normalized) = i.normalize_path(realm, ref_path, path) {
                let code = i.load_module(realm, normalized.as_str());
                return Some(Script::new(normalized.as_str(), code.as_str()));
            }
        }

        None
    }
}

impl Drop for QuickJsRuntimeAdapter {
    fn drop(&mut self) {
        // drop contexts first, should be done when Dropping EsRuntime?
        log::trace!("drop QuickJsRuntime, dropping contexts");

        self.contexts.clear();
        log::trace!("drop QuickJsRuntime, after dropping contexts");

        log::trace!("before JS_FreeRuntime");
        unsafe { q::JS_FreeRuntime(self.runtime) };
        log::trace!("after JS_FreeRuntime");
    }
}

impl JsRuntimeAdapter for QuickJsRuntimeAdapter {
    type JsRealmAdapterType = QuickJsRealmAdapter;
    type JsRuntimeFacadeType = QuickJsRuntimeFacade;

    fn js_load_module_script(&self, ref_path: &str, path: &str) -> Option<Script> {
        self.load_module_script_opt(ref_path, path)
    }

    fn js_create_realm(&self, _id: &str) -> Result<&Self::JsRealmAdapterType, JsError> {
        todo!()
        /*
                if self.js_get_realm(id).is_some() {

                    return Err(JsError::new_str("realm already exists"));
                }

                let ctx = QuickJsRealmAdapter::new(id.to_string(), self);

                self.contexts.insert(id.to_string(), ctx);

                let ctx = self.js_get_realm(id).expect("invalid state");
                let hooks = &*self.context_init_hooks.borrow();
                for hook in hooks {
                    hook(self, ctx)?;
                }

                Ok(ctx)
        */
    }

    fn js_remove_realm(&self, _id: &str) {
        todo!();
        //if !id.eq("__main__") {
        //            let _ = self.contexts.remove(id);
        //        }
    }

    fn js_get_realm(&self, id: &str) -> Option<&Self::JsRealmAdapterType> {
        if self.has_context(id) {
            Some(self.get_context(id))
        } else {
            None
        }
    }

    fn js_get_main_realm(&self) -> &Self::JsRealmAdapterType {
        self.get_main_context()
    }

    fn js_add_realm_init_hook<H>(&self, hook: H) -> Result<(), JsError>
    where
        H: Fn(&Self, &QuickJsRealmAdapter) -> Result<(), JsError> + 'static,
    {
        self.add_context_init_hook(hook)
    }
}

/// Helper for creating CStrings.
pub(crate) fn make_cstring(value: &str) -> Result<CString, JsError> {
    let res = CString::new(value);
    match res {
        Ok(val) => Ok(val),
        Err(_) => Err(JsError::new_string(format!(
            "could not create cstring from {}",
            value
        ))),
    }
}

#[cfg(test)]
pub mod tests {
    use crate::builder::QuickJsRuntimeBuilder;
    use crate::quickjsrealmadapter::QuickJsRealmAdapter;
    use crate::quickjsruntimeadapter::{QuickJsRuntimeAdapter, ScriptModuleLoader};

    use hirofa_utils::js_utils::adapters::{JsRealmAdapter, JsRuntimeAdapter};
    use hirofa_utils::js_utils::facades::{JsRuntimeBuilder, JsRuntimeFacade};
    use hirofa_utils::js_utils::Script;

    use std::panic;

    struct FooScriptModuleLoader {}
    impl ScriptModuleLoader<QuickJsRealmAdapter> for FooScriptModuleLoader {
        fn normalize_path(
            &self,
            _realm: &QuickJsRealmAdapter,
            _ref_path: &str,
            path: &str,
        ) -> Option<String> {
            Some(path.to_string())
        }

        fn load_module(&self, _realm: &QuickJsRealmAdapter, _absolute_path: &str) -> String {
            log::debug!("load_module");
            "{}".to_string()
        }
    }

    #[test]
    fn test_script_load() {
        log::debug!("testing1");
        let rt = QuickJsRuntimeBuilder::new()
            .script_module_loader(Box::new(FooScriptModuleLoader {}))
            .build();
        rt.exe_rt_task_in_event_loop(|q_js_rt| {
            log::debug!("testing2");
            let script = q_js_rt.load_module_script_opt("", "test.mjs").unwrap();
            assert_eq!(script.get_code(), "{}");
            log::debug!("tested");
        });
    }

    #[test]
    fn test_realm_init() {
        /*panic::set_hook(Box::new(|panic_info| {
                    let backtrace = Backtrace::new();
                    println!(
                        "thread panic occurred: {}\nbacktrace: {:?}",
                        panic_info, backtrace
                    );
                    log::error!(
                        "thread panic occurred: {}\nbacktrace: {:?}",
                        panic_info,
                        backtrace
                    );
                }));

                simple_logging::log_to_stderr(LevelFilter::max());
        */
        let rt = QuickJsRuntimeBuilder::new().js_build();

        rt.exe_task_in_event_loop(|| {
            QuickJsRuntimeAdapter::do_with(|rt| {
                rt.js_add_realm_init_hook(|_rt, realm| {
                    realm
                        .js_install_function(
                            &["utils"],
                            "doSomething",
                            |_rt, realm, _this, _args| realm.js_null_create(),
                            0,
                        )
                        .ok()
                        .expect("failed to install function");
                    match realm.eval(Script::new("t.js", "1+1")) {
                        Ok(_) => {}
                        Err(e) => {
                            panic!("script failed {}", e);
                        }
                    }
                    Ok(())
                })
                .ok()
                .expect("init hook addition failed");
            })
        });

        rt.js_loop_realm_void(Some("testrealm1"), |_rt, realm| {
            realm
                .eval(Script::new("test.js", "console.log(utils.doSomething());"))
                .ok()
                .expect("script failed");
        });
    }
}
