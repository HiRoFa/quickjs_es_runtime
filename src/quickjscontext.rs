use crate::quickjs_utils::{errors, functions, objects};
use crate::quickjsruntime::{make_cstring, QuickJsRuntime};
use crate::reflection::{Proxy, ProxyInstanceInfo};
use crate::valueref::{JSValueRef, TAG_EXCEPTION};
use hirofa_utils::auto_id_map::AutoIdMap;
use hirofa_utils::js_utils::adapters::{JsContextAdapter, JsRuntimeAdapter};
use hirofa_utils::js_utils::JsError;
use hirofa_utils::js_utils::Script;
use libquickjs_sys as q;
use std::cell::RefCell;
use std::collections::HashMap;
use std::ffi::CString;
use std::os::raw::c_void;
use std::rc::Rc;

type ProxyEventListenerMaps = HashMap<
    String, /*proxy_class_name*/
    HashMap<
        usize, /*proxy_instance_id*/
        HashMap<
            String, /*event_id*/
            HashMap<JSValueRef /*listener_func*/, JSValueRef /*options_obj*/>,
        >,
    >,
>;

pub struct QuickJsContext {
    object_cache: RefCell<AutoIdMap<JSValueRef>>,
    pub(crate) proxy_instance_id_mappings: RefCell<HashMap<usize, Box<ProxyInstanceInfo>>>,
    pub(crate) proxy_registry: RefCell<HashMap<String, Rc<Proxy>>>, // todo is this Rc needed or can we just borrow the Proxy when needed?
    pub(crate) proxy_event_listeners: RefCell<ProxyEventListenerMaps>,
    pub id: String,
    pub context: *mut q::JSContext,
}

thread_local! {
    static ID_REGISTRY: RefCell<HashMap<String, Box<String>>> = RefCell::new(HashMap::new());
}

impl QuickJsContext {
    pub(crate) fn free(&self) {
        log::trace!("QuickJsContext:free {}", self.id);
        {
            let cache_map = &mut *self.object_cache.borrow_mut();
            log::trace!(
                "QuickJsContext:free {}, dropping {} cached objects",
                self.id,
                cache_map.len()
            );
            cache_map.clear();
        }
        {
            let proxy_event_listeners = &mut *self.proxy_event_listeners.borrow_mut();
            proxy_event_listeners.clear();
        }

        unsafe { q::JS_FreeContext(self.context) };
        log::trace!("after QuickJsContext:free {}", self.id);
    }
    pub(crate) fn new(id: String, q_js_rt: &QuickJsRuntime) -> Self {
        let context = unsafe { q::JS_NewContext(q_js_rt.runtime) };

        let mut bx = Box::new(id.clone());

        let ibp: &mut String = &mut *bx;
        let info_ptr = ibp as *mut _ as *mut c_void;

        ID_REGISTRY.with(|rc| {
            let registry = &mut *rc.borrow_mut();
            registry.insert(id.clone(), bx);
        });

        unsafe { q::JS_SetContextOpaque(context, info_ptr) };

        if context.is_null() {
            panic!("ContextCreationFailed");
        }

        Self {
            id,
            context,
            object_cache: RefCell::new(AutoIdMap::new_with_max_size(i32::MAX as usize)),
            proxy_instance_id_mappings: RefCell::new(Default::default()),
            proxy_registry: RefCell::new(Default::default()),
            proxy_event_listeners: RefCell::new(Default::default()),
        }
    }
    /// get the id of a QuickJsContext from a JSContext
    /// # Safety
    /// when passing a context ptr please be sure that the corresponding QuickJsContext is still active
    pub unsafe fn get_id(context: *mut q::JSContext) -> &'static str {
        let info_ptr: *mut c_void = q::JS_GetContextOpaque(context);
        let info: &mut String = &mut *(info_ptr as *mut String);
        info
    }
    /// call a function by namespace and name
    pub fn call_function(
        &self,
        namespace: Vec<&str>,
        func_name: &str,
        arguments: Vec<JSValueRef>,
    ) -> Result<JSValueRef, JsError> {
        let namespace_ref = unsafe { objects::get_namespace(self.context, namespace, false) }?;
        functions::invoke_member_function_q(self, &namespace_ref, func_name, arguments)
    }
    /// evaluate a script

    pub fn eval(&self, script: Script) -> Result<JSValueRef, JsError> {
        unsafe { Self::eval_ctx(self.context, script) }
    }
    /// # Safety
    /// when passing a context ptr please be sure that the corresponding QuickJsContext is still active
    pub unsafe fn eval_ctx(
        context: *mut q::JSContext,
        mut script: Script,
    ) -> Result<JSValueRef, JsError> {
        log::debug!("q_js_rt.eval file {}", script.get_path());

        script = QuickJsRuntime::pre_process(script)?;

        let filename_c = make_cstring(script.get_path())?;
        let code_c = make_cstring(script.get_code())?;

        let value_raw = q::JS_Eval(
            context,
            code_c.as_ptr(),
            script.get_code().len() as _,
            filename_c.as_ptr(),
            q::JS_EVAL_TYPE_GLOBAL as i32,
        );

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
                log::debug!("eval_ctx failed: {}", ex);
                Err(ex)
            } else {
                Err(JsError::new_str("eval failed and could not get exception"))
            }
        } else {
            Ok(ret)
        }
    }

    /// evaluate a Module
    pub fn eval_module(&self, script: Script) -> Result<JSValueRef, JsError> {
        unsafe { Self::eval_module_ctx(self.context, script) }
    }

    /// # Safety
    /// when passing a context ptr please be sure that the corresponding QuickJsContext is still active
    pub unsafe fn eval_module_ctx(
        context: *mut q::JSContext,
        mut script: Script,
    ) -> Result<JSValueRef, JsError> {
        log::debug!("q_js_rt.eval_module file {}", script.get_path());

        script = QuickJsRuntime::pre_process(script)?;

        let filename_c = make_cstring(script.get_path())?;
        let code_c = make_cstring(script.get_code())?;

        let value_raw = q::JS_Eval(
            context,
            code_c.as_ptr(),
            script.get_code().len() as _,
            filename_c.as_ptr(),
            q::JS_EVAL_TYPE_MODULE as i32,
        );

        let ret = JSValueRef::new(
            context,
            value_raw,
            false,
            true,
            format!("eval_module result of {}", script.get_path()).as_str(),
        );

        log::trace!("evalled module yielded a {}", ret.borrow_value().tag);

        // check for error

        if ret.is_exception() {
            let ex_opt = Self::get_exception(context);
            if let Some(ex) = ex_opt {
                log::debug!("eval_module_ctx failed: {}", ex);
                Err(ex)
            } else {
                Err(JsError::new_str(
                    "eval_module failed and could not get exception",
                ))
            }
        } else {
            Ok(ret)
        }
    }
    /// throw an internal error to quickjs and create a new ex obj
    pub fn report_ex(&self, err: &str) -> q::JSValue {
        unsafe { Self::report_ex_ctx(self.context, err) }
    }
    /// throw an Error in the runtime and init an Exception JSValue to return
    /// # Safety
    /// when passing a context ptr please be sure that the corresponding QuickJsContext is still active
    pub unsafe fn report_ex_ctx(context: *mut q::JSContext, err: &str) -> q::JSValue {
        let c_err = CString::new(err);
        q::JS_ThrowInternalError(context, c_err.as_ref().ok().unwrap().as_ptr());
        q::JSValue {
            u: q::JSValueUnion { int32: 0 },
            tag: TAG_EXCEPTION,
        }
    }

    /// Get the last exception from the runtime, and if present, convert it to a JsError.
    pub fn get_exception_ctx(&self) -> Option<JsError> {
        unsafe { errors::get_exception(self.context) }
    }

    /// Get the last exception from the runtime, and if present, convert it to a JsError.
    /// # Safety
    /// When passing a context pointer please make sure the corresponding QuickJsContext is still valid
    pub unsafe fn get_exception(context: *mut q::JSContext) -> Option<JsError> {
        errors::get_exception(context)
    }

    pub fn cache_object(&self, obj: JSValueRef) -> i32 {
        let cache_map = &mut *self.object_cache.borrow_mut();
        let id = cache_map.insert(obj) as i32;
        log::trace!("cache_object: id={}, thread={}", id, thread_id::get());
        id
    }

    pub fn remove_cached_obj_if_present(&self, id: i32) {
        log::trace!(
            "remove_cached_obj_if_present: id={}, thread={}",
            id,
            thread_id::get()
        );
        let cache_map = &mut *self.object_cache.borrow_mut();
        if cache_map.contains_key(&(id as usize)) {
            let _ = cache_map.remove(&(id as usize));
        }
    }

    pub fn consume_cached_obj(&self, id: i32) -> JSValueRef {
        log::trace!("consume_cached_obj: id={}, thread={}", id, thread_id::get());
        let cache_map = &mut *self.object_cache.borrow_mut();
        cache_map.remove(&(id as usize))
    }

    pub fn with_cached_obj<C, R>(&self, id: i32, consumer: C) -> R
    where
        C: FnOnce(JSValueRef) -> R,
    {
        log::trace!("with_cached_obj: id={}, thread={}", id, thread_id::get());
        let clone_ref = {
            let cache_map = &*self.object_cache.borrow();
            let opt = cache_map.get(&(id as usize));
            let cached_ref = opt.expect("no such obj in cache");
            cached_ref.clone()
        };
        // prevent running consumer while borrowed

        consumer(clone_ref)
    }
    /// # Safety
    /// When passing a context pointer please make sure the corresponding QuickJsContext is still valid
    pub unsafe fn with_context<C, R>(context: *mut q::JSContext, consumer: C) -> R
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
        log::trace!("before drop QuickJSContext {}", self.id);

        let id = &self.id;
        {
            ID_REGISTRY.with(|rc| {
                let registry = &mut *rc.borrow_mut();
                registry.remove(id);
            });
        }
        {
            let proxies = &mut *self.proxy_registry.borrow_mut();
            proxies.clear();
        }
        {
            let id_mappings = &mut *self.proxy_instance_id_mappings.borrow_mut();
            id_mappings.clear();
        }

        log::trace!("after drop QuickJSContext {}", self.id);
    }
}

impl JsContextAdapter for QuickJsContext {
    type JsRuntimeAdapterType = QuickJsRuntime;

    fn js_eval(&self, script: Script) -> Result<JSValueRef, JsError> {
        self.eval(script)
    }

    fn js_install_function<
        F: Fn(
            <<Self as JsContextAdapter>::JsRuntimeAdapterType as JsRuntimeAdapter>::JsValueAdapterType,
            Vec<<<Self as JsContextAdapter>::JsRuntimeAdapterType as JsRuntimeAdapter>::JsValueAdapterType>,
        ) -> Result<<<Self as JsContextAdapter>::JsRuntimeAdapterType as JsRuntimeAdapter>::JsValueAdapterType, JsError>,
    >(&self, _namespace: Vec<&str>, _name: &str, _js_function: F, _arg_count: u32) -> Result<(), JsError>{
        todo!()
    }

    fn js_eval_module(&self, script: Script) -> Result<JSValueRef, JsError> {
        self.eval_module(script)
    }

    fn js_get_namespace(&self, _namespace: &[&str]) -> Result<JSValueRef, JsError> {
        todo!()
    }

    fn js_function_invoke(
        &self,
        namespace: &[&str],
        method_name: &str,
        args: &[&JSValueRef],
    ) -> Result<JSValueRef, JsError> {
        // todo see if we want to alter call_function
        let ns_vec = namespace.to_vec();
        let args_vec = args.to_vec().into_iter().cloned().collect();

        self.call_function(ns_vec, method_name, args_vec)
    }

    fn js_function_invoke2(
        &self,
        _this_obj: &JSValueRef,
        _method_name: &str,
        _args: &[&JSValueRef],
    ) -> Result<JSValueRef, JsError> {
        todo!()
    }

    fn js_function_invoke3(
        &self,
        _this_obj: Option<&JSValueRef>,
        _function_obj: &JSValueRef,
        _args: &[&JSValueRef],
    ) -> Result<JSValueRef, JsError> {
        todo!()
    }

    fn js_object_delete_property(
        &self,
        _object: &JSValueRef,
        _property_name: &str,
    ) -> Result<(), JsError> {
        todo!()
    }

    fn js_object_set_property(
        &self,
        object: &JSValueRef,
        property_name: &str,
        property: &JSValueRef,
    ) -> Result<(), JsError> {
        objects::set_property_q(self, object, property_name, property)
    }

    fn js_object_get_property(
        &self,
        object: &JSValueRef,
        property_name: &str,
    ) -> Result<JSValueRef, JsError> {
        objects::get_property_q(self, object, property_name)
    }

    fn js_object_create_new(&self) -> Result<JSValueRef, JsError> {
        todo!()
    }

    fn js_object_get_properties(&self, object: &JSValueRef) -> Result<Vec<String>, JsError> {
        let props = objects::get_own_property_names_q(self, object)?;
        let mut ret = vec![];
        for x in 0..props.len() {
            let prop = props.get_name(x)?;
            ret.push(prop);
        }
        Ok(ret)
    }

    fn js_array_get_element(
        &self,
        _array: &JSValueRef,
        _index: u32,
    ) -> Result<JSValueRef, JsError> {
        todo!()
    }

    fn js_array_set_element(
        &self,
        _array: &JSValueRef,
        _index: u32,
        _element: JSValueRef,
    ) -> Result<(), JsError> {
        todo!()
    }

    fn js_array_get_length(&self, _array: &JSValueRef) -> Result<u32, JsError> {
        todo!()
    }

    fn js_array_create_new(&self) -> Result<JSValueRef, JsError> {
        todo!()
    }
}

#[cfg(test)]
pub mod tests {
    use crate::esruntime::tests::init_test_rt;
    use crate::esruntimebuilder::EsRuntimeBuilder;
    use crate::quickjs_utils;
    use crate::quickjs_utils::primitives::to_i32;
    use crate::quickjs_utils::{functions, get_global_q, objects};
    use hirofa_utils::js_utils::Script;

    #[test]
    fn test_eval() {
        let rt = init_test_rt();
        rt.exe_rt_task_in_event_loop(|q_js_rt| {
            let q_ctx = q_js_rt.get_main_context();
            let res = q_ctx.eval(Script::new("test_eval.es", "(1 + 1);"));

            match res {
                Ok(res) => {
                    log::info!("script ran ok: {:?}", res);
                    assert!(res.is_i32());
                    assert_eq!(to_i32(&res).ok().expect("conversion failed"), 2);
                }
                Err(e) => {
                    log::error!("script failed: {}", e);
                    panic!("script failed");
                }
            }
        });
    }

    #[test]
    fn test_multi_ctx() {
        let rt = EsRuntimeBuilder::new().build();
        rt.create_context("a").ok().expect("could not create ctx a");
        rt.create_context("b").ok().expect("could not create ctx b");

        rt.exe_rt_task_in_event_loop(|q_js_rt| {
            let ctx_a = q_js_rt.get_context("a");
            let ctx_b = q_js_rt.get_context("b");
            ctx_a
                .eval(Script::new("a.es", "this.a = 1"))
                .ok()
                .expect("script failed");
            ctx_b
                .eval(Script::new("a.es", "this.b = 1"))
                .ok()
                .expect("script failed");
            let v = ctx_a
                .eval(Script::new("a2.es", "this.a;"))
                .ok()
                .expect("script failed");
            assert!(v.is_i32());
            let v2 = ctx_b
                .eval(Script::new("b2.es", "this.a;"))
                .ok()
                .expect("script failed");
            assert!(v2.is_null_or_undefined());
            let v3 = ctx_a
                .eval(Script::new("a2.es", "this.b;"))
                .ok()
                .expect("script failed");
            assert!(v3.is_null_or_undefined());
            let v4 = ctx_b
                .eval(Script::new("b2.es", "this.b;"))
                .ok()
                .expect("script failed");
            assert!(v4.is_i32());
        });
        rt.drop_context("b");

        rt.exe_rt_task_in_event_loop(|q_js_rt| {
            q_js_rt.gc();
            let ctx_a = q_js_rt.get_context("a");
            let v = ctx_a
                .eval(Script::new("a2.es", "this.a;"))
                .ok()
                .expect("script failed");
            assert!(v.is_i32());
            q_js_rt.gc();
        });

        rt.create_context("c")
            .ok()
            .expect("could not create context c");

        rt.exe_rt_task_in_event_loop(|q_js_rt| {
            let c_ctx = q_js_rt.get_context("c");
            let func = functions::new_function_q(
                c_ctx,
                "test",
                |_q_ctx, _this, _args| Ok(quickjs_utils::new_null_ref()),
                1,
            )
            .ok()
            .unwrap();
            let global = get_global_q(c_ctx);
            objects::set_property_q(c_ctx, &global, "test_func", &func)
                .ok()
                .expect("could not set prop");
            q_js_rt.gc();
        });
        rt.exe_rt_task_in_event_loop(|q_js_rt| {
            q_js_rt.gc();
            let ctx_a = q_js_rt.get_context("a");
            let v = ctx_a
                .eval(Script::new("a2.es", "this.a;"))
                .ok()
                .expect("script failed");
            assert!(v.is_i32());
            q_js_rt.gc();
        });
        rt.drop_context("c");
        rt.exe_rt_task_in_event_loop(|q_js_rt| {
            q_js_rt.gc();
            let ctx_a = q_js_rt.get_context("a");
            let v = ctx_a
                .eval(Script::new("a2.es", "this.a;"))
                .ok()
                .expect("script failed");
            assert!(v.is_i32());
            q_js_rt.gc();
        });
    }
}
