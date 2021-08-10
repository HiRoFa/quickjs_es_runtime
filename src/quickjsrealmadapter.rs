use crate::quickjs_utils::primitives::{from_bool, from_f64, from_i32, from_string_q};
use crate::quickjs_utils::promises::PromiseRef;
use crate::quickjs_utils::{arrays, errors, functions, json, new_null_ref, objects};
use crate::quickjsruntimeadapter::{make_cstring, QuickJsRuntimeAdapter};
use crate::reflection::{Proxy, ProxyInstanceInfo};
use crate::valueref::{JSValueRef, TAG_EXCEPTION};
use hirofa_utils::auto_id_map::AutoIdMap;
use hirofa_utils::js_utils::adapters::proxies::{JsProxy, JsProxyMember, JsProxyStaticMember};
use hirofa_utils::js_utils::adapters::{JsRealmAdapter, JsValueAdapter};
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

pub struct QuickJsRealmAdapter {
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

impl QuickJsRealmAdapter {
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
    pub(crate) fn new(id: String, q_js_rt: &QuickJsRuntimeAdapter) -> Self {
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

        script = QuickJsRuntimeAdapter::pre_process(script)?;

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

        script = QuickJsRuntimeAdapter::pre_process(script)?;

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
        C: FnOnce(&QuickJsRealmAdapter) -> R,
    {
        QuickJsRuntimeAdapter::do_with(|q_js_rt| {
            let id = QuickJsRealmAdapter::get_id(context);
            let q_ctx = q_js_rt.get_context(id);
            consumer(q_ctx)
        })
    }
}

impl Drop for QuickJsRealmAdapter {
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

impl JsRealmAdapter for QuickJsRealmAdapter {
    type JsRuntimeAdapterType = QuickJsRuntimeAdapter;
    type JsValueAdapterType = JSValueRef;
    type JsPromiseAdapterType = PromiseRef;

    fn js_get_script_or_module_name(&self) -> Result<String, JsError> {
        crate::quickjs_utils::get_script_or_module_name_q(self)
    }

    fn js_eval(&self, script: Script) -> Result<JSValueRef, JsError> {
        self.eval(script)
    }

    fn js_proxy_install(
        &self,
        mut proxy: JsProxy<QuickJsRealmAdapter>,
    ) -> Result<JSValueRef, JsError> {
        // create qjs proxy from proxy
        let mut q_proxy = Proxy::new();

        // todo revam qjs proxy to have rt as first arg in methods/getter/setters etc
        if let Some(constructor) = proxy.constructor.take() {
            q_proxy = q_proxy.constructor(move |realm, id, args| {
                QuickJsRuntimeAdapter::do_with(|rt| constructor(rt, realm, &id, args.as_slice()))
            });
        }
        if let Some(finalizer) = proxy.finalizer.take() {
            q_proxy = q_proxy.finalizer(move |realm, id| {
                QuickJsRuntimeAdapter::do_with(|rt| finalizer(rt, realm, &id))
            });
        }
        for member in proxy.members {
            match member.1 {
                JsProxyMember::Method { method } => {
                    q_proxy = q_proxy.method(member.0, move |realm, id, args| {
                        //
                        QuickJsRuntimeAdapter::do_with(|rt| method(rt, realm, id, args.as_slice()))
                    })
                }
                JsProxyMember::GetterSetter { get, set } => {
                    q_proxy = q_proxy.getter_setter(
                        member.0,
                        move |realm, id| QuickJsRuntimeAdapter::do_with(|rt| get(rt, realm, id)),
                        move |realm, id, val| {
                            QuickJsRuntimeAdapter::do_with(|rt| set(rt, realm, id, &val))
                        },
                    );
                }
            }
        }
        for static_member in proxy.static_members {
            match static_member.1 {
                JsProxyStaticMember::StaticMethod { method } => {
                    q_proxy = q_proxy.static_method(static_member.0, move |realm, args| {
                        //
                        QuickJsRuntimeAdapter::do_with(|rt| method(rt, realm, args.as_slice()))
                    })
                }
                JsProxyStaticMember::StaticGetterSetter { get, set } => {
                    q_proxy = q_proxy.static_getter_setter(
                        static_member.0,
                        move |realm| QuickJsRuntimeAdapter::do_with(|rt| get(rt, realm)),
                        move |realm, val| QuickJsRuntimeAdapter::do_with(|rt| set(rt, realm, &val)),
                    );
                }
            }
        }

        // todo.. eventhandlers should not be in JsProxy at all should they?
        //

        q_proxy.install(self, true)
    }

    fn js_proxy_instantiate(
        &self,
        _namespace: &[&str],
        _class_name: &str,
        _arguments: &[JSValueRef],
    ) -> Result<JSValueRef, JsError> {
        unimplemented!()
    }

    fn js_proxy_invoke_event(
        &self,
        _proxy_handle: &usize,
        _event_id: &str,
        _event_obj: &JSValueRef,
    ) {
        unimplemented!()
    }

    fn js_install_function(
        &self,
        namespace: &[&str],
        name: &str,
        js_function: fn(
            &QuickJsRuntimeAdapter,
            &Self,
            &JSValueRef,
            &[JSValueRef],
        ) -> Result<JSValueRef, JsError>,
        arg_count: u32,
    ) -> Result<(), JsError> {
        // todo namespace as slice?
        let ns = self.js_get_namespace(namespace)?;

        let func = functions::new_function_q(
            self,
            name,
            move |ctx, this, args| {
                QuickJsRuntimeAdapter::do_with(|rt| js_function(rt, ctx, this, args))
            },
            arg_count,
        )?;
        self.js_object_set_property(&ns, name, &func)?;
        Ok(())
    }

    fn js_install_closure<
        F: Fn(
                &QuickJsRuntimeAdapter,
                &Self,
                &JSValueRef,
                &[JSValueRef],
            ) -> Result<JSValueRef, JsError>
            + 'static,
    >(
        &self,
        namespace: &[&str],
        name: &str,
        js_function: F,
        arg_count: u32,
    ) -> Result<(), JsError> {
        // todo namespace as slice?
        let ns = self.js_get_namespace(namespace)?;

        let func = functions::new_function_q(
            self,
            name,
            move |ctx, this, args| {
                QuickJsRuntimeAdapter::do_with(|rt| js_function(rt, ctx, this, args))
            },
            arg_count,
        )?;
        self.js_object_set_property(&ns, name, &func)?;
        Ok(())
    }

    fn js_eval_module(&self, script: Script) -> Result<JSValueRef, JsError> {
        self.eval_module(script)
    }

    fn js_get_namespace(&self, namespace: &[&str]) -> Result<JSValueRef, JsError> {
        let namespace_vec = namespace.to_vec();
        objects::get_namespace_q(self, namespace_vec, true)
    }

    fn js_function_invoke_by_name(
        &self,
        namespace: &[&str],
        method_name: &str,
        args: &[JSValueRef],
    ) -> Result<JSValueRef, JsError> {
        // todo see if we want to alter call_function
        let ns_vec = namespace.to_vec();
        let args_vec = args.to_vec();

        self.call_function(ns_vec, method_name, args_vec)
    }

    fn js_function_invoke_member_by_name(
        &self,
        this_obj: &JSValueRef,
        method_name: &str,
        args: &[JSValueRef],
    ) -> Result<JSValueRef, JsError> {
        let args_vec = args.to_vec();
        functions::invoke_member_function_q(self, this_obj, method_name, args_vec)
    }

    fn js_function_invoke(
        &self,
        this_obj: Option<&JSValueRef>,
        function_obj: &JSValueRef,
        args: &[JSValueRef],
    ) -> Result<JSValueRef, JsError> {
        let args_vec = args.to_vec();
        functions::call_function_q(self, function_obj, args_vec, this_obj)
    }

    fn js_function_create<
        F: Fn(&Self, &JSValueRef, &[JSValueRef]) -> Result<JSValueRef, JsError> + 'static,
    >(
        &self,
        name: &str,
        js_function: F,
        arg_count: u32,
    ) -> Result<JSValueRef, JsError> {
        functions::new_function_q(self, name, js_function, arg_count)
    }

    fn js_object_delete_property(
        &self,
        object: &JSValueRef,
        property_name: &str,
    ) -> Result<(), JsError> {
        // todo impl a real delete_prop
        objects::set_property_q(self, object, property_name, &new_null_ref())
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

    fn js_object_create(&self) -> Result<JSValueRef, JsError> {
        objects::create_object_q(self)
    }

    fn js_object_construct(
        &self,
        _constructor: &JSValueRef,
        _args: &[JSValueRef],
    ) -> Result<JSValueRef, JsError> {
        unimplemented!()
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

    fn js_object_traverse<F, R>(&self, object: &JSValueRef, visitor: F) -> Result<Vec<R>, JsError>
    where
        F: Fn(&str, &JSValueRef) -> Result<R, JsError>,
    {
        objects::traverse_properties_q(self, object, visitor)
    }

    fn js_array_get_element(&self, array: &JSValueRef, index: u32) -> Result<JSValueRef, JsError> {
        arrays::get_element_q(self, array, index)
    }

    fn js_array_set_element(
        &self,
        array: &JSValueRef,
        index: u32,
        element: JSValueRef,
    ) -> Result<(), JsError> {
        arrays::set_element_q(self, array, index, element)
    }

    fn js_array_get_length(&self, array: &JSValueRef) -> Result<u32, JsError> {
        arrays::get_length_q(self, array)
    }

    fn js_array_create(&self) -> Result<JSValueRef, JsError> {
        arrays::create_array_q(self)
    }

    fn js_array_traverse<F, R>(&self, array: &JSValueRef, visitor: F) -> Result<Vec<R>, JsError>
    where
        F: Fn(u32, &JSValueRef) -> Result<R, JsError>,
    {
        // todo impl real traverse methods
        let mut ret = vec![];
        for x in 0..arrays::get_length_q(self, array)? {
            let val = arrays::get_element_q(self, array, x)?;
            ret.push(visitor(x, &val)?)
        }
        Ok(ret)
    }

    fn js_null_create(&self) -> Result<JSValueRef, JsError> {
        Ok(crate::quickjs_utils::new_null_ref())
    }

    fn js_undefined_create(&self) -> Result<JSValueRef, JsError> {
        Ok(crate::quickjs_utils::new_undefined_ref())
    }

    fn js_i32_create(&self, val: i32) -> Result<JSValueRef, JsError> {
        Ok(from_i32(val))
    }

    fn js_string_create(&self, val: &str) -> Result<JSValueRef, JsError> {
        from_string_q(self, val)
    }

    fn js_boolean_create(&self, val: bool) -> Result<JSValueRef, JsError> {
        Ok(from_bool(val))
    }

    fn js_f64_create(&self, val: f64) -> Result<JSValueRef, JsError> {
        Ok(from_f64(val))
    }

    fn js_promise_create(&self) -> Result<Box<PromiseRef>, JsError> {
        Ok(Box::new(crate::quickjs_utils::promises::new_promise_q(
            self,
        )?))
    }

    fn js_cache_add(&self, object: JSValueRef) -> i32 {
        self.cache_object(object)
    }

    fn js_cache_dispose(&self, id: i32) {
        let _ = self.consume_cached_obj(id);
    }

    fn js_cache_with<C, R>(&self, id: i32, consumer: C) -> R
    where
        C: FnOnce(&JSValueRef) -> R,
    {
        self.with_cached_obj(id, |obj| consumer(&obj))
    }

    fn js_cache_consume(&self, id: i32) -> JSValueRef {
        self.consume_cached_obj(id)
    }

    fn js_instance_of(&self, object: &JSValueRef, constructor: &JSValueRef) -> bool {
        objects::is_instance_of_q(self, object, constructor)
    }

    fn js_json_stringify(
        &self,
        object: &JSValueRef,
        opt_space: Option<&str>,
    ) -> Result<String, JsError> {
        let opt_space_jsvr = match opt_space {
            None => None,
            Some(s) => Some(self.js_string_create(s)?),
        };
        let res = json::stringify_q(self, object, opt_space_jsvr);
        match res {
            Ok(jsvr) => jsvr.js_to_string(),
            Err(e) => Err(e),
        }
    }

    fn js_json_parse(&self, json_string: &str) -> Result<JSValueRef, JsError> {
        json::parse_q(self, json_string)
    }
}

#[cfg(test)]
pub mod tests {
    use crate::builder::QuickJsRuntimeBuilder;
    use crate::facades::tests::init_test_rt;
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
        let rt = QuickJsRuntimeBuilder::new().build();
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
