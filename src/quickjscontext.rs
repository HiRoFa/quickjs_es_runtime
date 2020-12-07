use crate::eserror::EsError;
use crate::esscript::EsScript;
use crate::quickjs_utils::{errors, functions, objects};
use crate::quickjsruntime::{make_cstring, QuickJsRuntime};
use crate::reflection::{Proxy, ProxyInstanceInfo};
use crate::valueref::{JSValueRef, TAG_EXCEPTION};
use hirofa_utils::auto_id_map::AutoIdMap;
use libquickjs_sys as q;
use std::cell::RefCell;
use std::collections::HashMap;
use std::ffi::CString;
use std::os::raw::c_void;
use std::sync::Arc;

pub struct QuickJsContext {
    object_cache: RefCell<AutoIdMap<JSValueRef>>,
    pub(crate) instance_id_mappings: RefCell<HashMap<usize, Box<ProxyInstanceInfo>>>,
    pub(crate) proxy_registry: RefCell<HashMap<String, Arc<Proxy>>>, // todo: why do we need an Arc around proxy and not an Rc?
    pub id: String,
    pub context: *mut q::JSContext,
}

thread_local! {
    static ID_REGISTRY: RefCell<HashMap<String, Box<String>>> = RefCell::new(HashMap::new());
}

impl QuickJsContext {
    pub(crate) fn new(id: String, q_js_rt: &QuickJsRuntime) -> Self {
        let mut bx = Box::new(id.clone());

        let ibp: &mut String = &mut *bx;
        let info_ptr = ibp as *mut _ as *mut c_void;

        ID_REGISTRY.with(|rc| {
            let registry = &mut *rc.borrow_mut();
            registry.insert(id.clone(), bx);
        });

        let context = unsafe { q::JS_NewContext(q_js_rt.runtime) };

        unsafe { q::JS_SetContextOpaque(context, info_ptr) };

        if context.is_null() {
            panic!("ContextCreationFailed");
        }

        Self {
            id,
            context,
            object_cache: RefCell::new(AutoIdMap::new_with_max_size(i32::MAX as usize)),
            instance_id_mappings: RefCell::new(HashMap::new()),
            proxy_registry: RefCell::new(HashMap::new()),
        }
    }
    pub fn get_id(context: *mut q::JSContext) -> &'static str {
        let info_ptr: *mut c_void = unsafe { q::JS_GetContextOpaque(context) };
        let info: &mut String = unsafe { &mut *(info_ptr as *mut String) };
        info
    }
    /// call a function by namespace and name
    pub fn call_function(
        &self,
        namespace: Vec<&str>,
        func_name: &str,
        arguments: Vec<JSValueRef>,
    ) -> Result<JSValueRef, EsError> {
        let namespace_ref = objects::get_namespace(self.context, namespace, false)?;
        functions::invoke_member_function(self.context, &namespace_ref, func_name, arguments)
    }
    /// evaluate a script

    pub fn eval(&self, script: EsScript) -> Result<JSValueRef, EsError> {
        Self::eval_ctx(self.context, script)
    }
    pub fn eval_ctx(context: *mut q::JSContext, script: EsScript) -> Result<JSValueRef, EsError> {
        let filename_c = make_cstring(script.get_path())?;
        let code_c = make_cstring(script.get_code())?;

        log::debug!("q_js_rt.eval file {}", script.get_path());

        let value_raw = unsafe {
            q::JS_Eval(
                context,
                code_c.as_ptr(),
                script.get_code().len() as _,
                filename_c.as_ptr(),
                q::JS_EVAL_TYPE_GLOBAL as i32,
            )
        };

        log::trace!("after eval, checking error");

        // check for error
        let ret = JSValueRef::new(
            context,
            value_raw,
            false,
            true,
            format!("eval result of {}", script.get_path()).as_str(),
        );
        if ret.is_exception() {
            let ex_opt = Self::get_exception(context);
            if let Some(ex) = ex_opt {
                Err(ex)
            } else {
                Err(EsError::new_str("eval failed and could not get exception"))
            }
        } else {
            // this sucks (kills some async stuff and performance) but prevents stack overflows when doing stuff with promises which are not referenced after eval
            // (pending jobs should be taken care of in separate job on event queue which is always added)
            // i'm not quite sure why that happens yet
            // p.s. same in eval_module
            QuickJsRuntime::do_with(|q_js_rt| {
                while q_js_rt.has_pending_jobs() {
                    q_js_rt.run_pending_job()?;
                }
                Ok(())
            })?;

            Ok(ret)
        }
    }

    /// evaluate a Module
    pub fn eval_module(&self, script: EsScript) -> Result<JSValueRef, EsError> {
        Self::eval_module_ctx(self.context, script)
    }
    pub fn eval_module_ctx(
        context: *mut q::JSContext,
        script: EsScript,
    ) -> Result<JSValueRef, EsError> {
        log::debug!("q_js_rt.eval_module file {}", script.get_path());

        let filename_c = make_cstring(script.get_path())?;
        let code_c = make_cstring(script.get_code())?;

        let value_raw = unsafe {
            q::JS_Eval(
                context,
                code_c.as_ptr(),
                script.get_code().len() as _,
                filename_c.as_ptr(),
                q::JS_EVAL_TYPE_MODULE as i32,
            )
        };

        // check for error
        let ret = JSValueRef::new(
            context,
            value_raw,
            false,
            true,
            format!("eval_module result of {}", script.get_path()).as_str(),
        );

        log::trace!("evalled module yielded a {}", ret.borrow_value().tag);

        if ret.is_exception() {
            let ex_opt = Self::get_exception(context);
            if let Some(ex) = ex_opt {
                Err(ex)
            } else {
                Err(EsError::new_str(
                    "eval_module failed and could not get exception",
                ))
            }
        } else {
            QuickJsRuntime::do_with(|q_js_rt| {
                while q_js_rt.has_pending_jobs() {
                    q_js_rt.run_pending_job()?;
                }
                Ok(())
            })?;
            Ok(ret)
        }
    }
    /// throw an internal error to quickjs and create a new ex obj
    pub fn report_ex(&self, err: &str) -> q::JSValue {
        Self::report_ex_ctx(self.context, err)
    }
    pub fn report_ex_ctx(context: *mut q::JSContext, err: &str) -> q::JSValue {
        let c_err = CString::new(err);
        unsafe { q::JS_ThrowInternalError(context, c_err.as_ref().ok().unwrap().as_ptr()) };
        q::JSValue {
            u: q::JSValueUnion { int32: 0 },
            tag: TAG_EXCEPTION,
        }
    }

    /// Get the last exception from the runtime, and if present, convert it to a EsError.
    pub fn get_exception(context: *mut q::JSContext) -> Option<EsError> {
        errors::get_exception(context)
    }

    pub fn cache_object(&self, obj: JSValueRef) -> i32 {
        let cache_map = &mut *self.object_cache.borrow_mut();
        let id = cache_map.insert(obj) as i32;
        log::trace!("cache_object: id={}, thread={}", id, thread_id::get());
        id
    }

    pub fn consume_cached_obj(&self, id: i32) -> JSValueRef {
        log::trace!("consume_cached_obj: id={}, thread={}", id, thread_id::get());
        let cache_map = &mut *self.object_cache.borrow_mut();
        cache_map.remove(&(id as usize))
    }

    pub fn with_cached_obj<C, R>(&self, id: i32, consumer: C) -> R
    where
        C: FnOnce(&JSValueRef) -> R,
    {
        log::trace!("with_cached_obj: id={}, thread={}", id, thread_id::get());
        let cache_map = &*self.object_cache.borrow();
        let opt = cache_map.get(&(id as usize));
        consumer(opt.expect("no such obj in cache"))
    }

    pub fn with_context<C, R>(context: *mut q::JSContext, consumer: C) -> R
    where
        C: FnOnce(&QuickJsContext) -> R,
    {
        QuickJsRuntime::do_with(|q_js_rt| {
            let id = QuickJsContext::get_id(context);
            let q_ctx = q_js_rt.get_context(id);
            consumer(q_ctx)
        })
    }
}

impl Drop for QuickJsContext {
    fn drop(&mut self) {
        // free ctx
        log::trace!("before JS_FreeContext");

        {
            let cache_map = &mut *self.object_cache.borrow_mut();
            cache_map.remove_values(|_v| true);
        }
        unsafe { q::JS_FreeContext(self.context) };
    }
}

#[cfg(test)]
pub mod tests {
    use crate::esruntimebuilder::EsRuntimeBuilder;
    use crate::esscript::EsScript;

    #[test]
    fn test_multi_ctx() {
        let rt = EsRuntimeBuilder::new().build();
        rt.add_to_event_queue_mut_sync(|q_js_rt| {
            q_js_rt
                .create_context("a")
                .ok()
                .expect("could not create ctx a");
            q_js_rt
                .create_context("b")
                .ok()
                .expect("could not create ctx b");
        });
        rt.add_to_event_queue_sync(|q_js_rt| {
            let ctx_a = q_js_rt.get_context("a");
            let ctx_b = q_js_rt.get_context("b");
            ctx_a
                .eval(EsScript::new("a.es", "this.a = 1"))
                .ok()
                .expect("script failed");
            ctx_b
                .eval(EsScript::new("a.es", "this.b = 1"))
                .ok()
                .expect("script failed");
            let v = ctx_a
                .eval(EsScript::new("a2.es", "this.a;"))
                .ok()
                .expect("script failed");
            assert!(v.is_i32());
            let v2 = ctx_b
                .eval(EsScript::new("b2.es", "this.a;"))
                .ok()
                .expect("script failed");
            assert!(v2.is_null_or_undefined());
        });
        rt.add_to_event_queue_mut_sync(|q_js_rt| {
            q_js_rt.drop_context("b");
        });
        rt.add_to_event_queue_sync(|q_js_rt| {
            q_js_rt.gc();
            let ctx_a = q_js_rt.get_context("a");
            let v = ctx_a
                .eval(EsScript::new("a2.es", "this.a;"))
                .ok()
                .expect("script failed");
            assert!(v.is_i32());
            q_js_rt.gc();
        });
        rt.add_to_event_queue_mut_sync(|q_js_rt| {
            q_js_rt
                .create_context("c")
                .ok()
                .expect("could not create context c");
        });
    }
}
