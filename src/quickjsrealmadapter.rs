use crate::facades::QuickjsRuntimeFacadeInner;
use crate::quickjs_utils::objects::construct_object;
use crate::quickjs_utils::primitives::{from_bool, from_f64, from_i32, from_string_q};
use crate::quickjs_utils::typedarrays::{
    detach_array_buffer_buffer_q, get_array_buffer_buffer_copy_q, get_array_buffer_q,
    new_uint8_array_copy_q, new_uint8_array_q,
};
use crate::quickjs_utils::{arrays, errors, functions, get_global_q, json, new_null_ref, objects};
use crate::quickjsruntimeadapter::{make_cstring, QuickJsRuntimeAdapter};
use crate::quickjsvalueadapter::{QuickJsValueAdapter, TAG_EXCEPTION};
use crate::reflection::eventtarget::dispatch_event;
use crate::reflection::eventtarget::dispatch_static_event;
use crate::reflection::{new_instance, new_instance3, Proxy};
use hirofa_utils::auto_id_map::AutoIdMap;

use crate::jsutils::jsproxies::{JsProxy, JsProxyInstanceId};
use crate::jsutils::{JsError, JsValueType, Script};
use crate::quickjs_utils::promises::QuickJsPromiseAdapter;
use crate::values::{
    CachedJsArrayRef, CachedJsFunctionRef, CachedJsObjectRef, CachedJsPromiseRef, JsValueFacade,
    TypedArrayType,
};
use libquickjs_sys as q;
use serde_json::Value;
use std::cell::RefCell;
use std::collections::HashMap;
use std::ffi::CString;
use std::future::Future;
use std::os::raw::c_void;
use std::rc::Rc;
use std::sync::{Arc, Weak};

use crate::jsutils::promises::new_resolving_promise;
use crate::jsutils::promises::new_resolving_promise_async;
use string_cache::DefaultAtom;

type ProxyEventListenerMaps = HashMap<
    String, /*proxy_class_name*/
    HashMap<
        usize, /*proxy_instance_id*/
        HashMap<
            String, /*event_id*/
            HashMap<
                QuickJsValueAdapter, /*listener_func*/
                QuickJsValueAdapter, /*options_obj*/
            >,
        >,
    >,
>;

type ProxyStaticEventListenerMaps = HashMap<
    String, /*proxy_class_name*/
    HashMap<
        String, /*event_id*/
        HashMap<
            QuickJsValueAdapter, /*listener_func*/
            QuickJsValueAdapter, /*options_obj*/
        >,
    >,
>;

pub struct QuickJsRealmAdapter {
    object_cache: RefCell<AutoIdMap<QuickJsValueAdapter>>,
    promise_cache: RefCell<AutoIdMap<QuickJsPromiseAdapter>>,
    pub(crate) proxy_registry: RefCell<HashMap<String, Rc<Proxy>>>, // todo is this Rc needed or can we just borrow the Proxy when needed?
    pub(crate) proxy_constructor_refs: RefCell<HashMap<String, QuickJsValueAdapter>>,
    pub(crate) proxy_event_listeners: RefCell<ProxyEventListenerMaps>,
    pub(crate) proxy_static_event_listeners: RefCell<ProxyStaticEventListenerMaps>,
    pub id: String,
    pub context: *mut q::JSContext,
}

thread_local! {
    #[allow(clippy::box_collection)]
    static ID_REGISTRY: RefCell<HashMap<String, Box<String>>> = RefCell::new(HashMap::new());
}

impl QuickJsRealmAdapter {
    pub fn print_stats(&self) {
        println!(
            "QuickJsRealmAdapter.object_cache.len = {}",
            self.object_cache.borrow().len()
        );
        println!(
            "QuickJsRealmAdapter.promise_cache.len = {}",
            self.promise_cache.borrow().len()
        );

        println!("-- > QuickJsRealmAdapter.proxy instances");
        for p in &*self.proxy_registry.borrow() {
            let prc = p.1.clone();
            let proxy = &*prc;
            let mappings = &*proxy.proxy_instance_id_mappings.borrow();
            println!("---- > {} len:{}", p.0, mappings.len());
            print!("------ ids: ");
            for i in mappings {
                print!("{}, ", i.0);
            }
            println!("\n---- < {}", p.0);
        }
        println!("-- < QuickJsRealmAdapter.proxy instances");

        let _spsel: &ProxyStaticEventListenerMaps = &self.proxy_static_event_listeners.borrow();
        let psel: &ProxyEventListenerMaps = &self.proxy_event_listeners.borrow();

        println!("> psel");
        for a in psel {
            println!("- psel - {}", a.0);
            let map = a.1;
            for b in map {
                println!("- psel - id {}", b.0);
                let map_b = b.1;
                for c in map_b {
                    println!("- psel - id {} - evt {}", b.0, c.0);
                    let map_c = c.1;
                    println!(
                        "- psel - id {} - evt {} - mapC.len={}",
                        b.0,
                        c.0,
                        map_c.len()
                    );
                    for eh in map_c {
                        // handler, options?
                        println!(
                            "- psel - id {} - evt {} - handler:{} options:{}",
                            b.0,
                            c.0,
                            eh.0.to_string().expect("could not toString"),
                            eh.1.to_string().expect("could not toString")
                        );
                    }
                }
            }
        }
        println!("< psel");
    }

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

        let mut all_listeners = {
            let proxy_event_listeners: &mut ProxyEventListenerMaps =
                &mut self.proxy_event_listeners.borrow_mut();
            std::mem::take(proxy_event_listeners)
        };
        // drop outside of borrowmut so finalizers don;t get error when trying to get mut borrow on map
        all_listeners.clear();

        // hmm these should still exist minus the constrcutor ref on free, so we need to remove the constructor refs, then call free, then call gc and then clear proxies
        // so here we should just clear the refs..
        let mut all_constructor_refs = {
            let proxy_constructor_refs = &mut *self.proxy_constructor_refs.borrow_mut();
            std::mem::take(proxy_constructor_refs)
        };
        all_constructor_refs.clear();

        unsafe { q::JS_FreeContext(self.context) };

        log::trace!("after QuickJsContext:free {}", self.id);
    }
    pub(crate) fn new(id: String, q_js_rt: &QuickJsRuntimeAdapter) -> Self {
        let context = unsafe { q::JS_NewContext(q_js_rt.runtime) };

        let mut bx = Box::new(id.clone());

        let ibp: &mut String = &mut bx;
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
            promise_cache: RefCell::new(AutoIdMap::new()),
            proxy_registry: RefCell::new(Default::default()),
            proxy_constructor_refs: RefCell::new(Default::default()),
            proxy_event_listeners: RefCell::new(Default::default()),
            proxy_static_event_listeners: RefCell::new(Default::default()),
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
    /// invoke a function by namespace and name
    pub fn invoke_function_by_name(
        &self,
        namespace: &[&str],
        func_name: &str,
        arguments: &[QuickJsValueAdapter],
    ) -> Result<QuickJsValueAdapter, JsError> {
        let namespace_ref = unsafe { objects::get_namespace(self.context, namespace, false) }?;
        functions::invoke_member_function_q(self, &namespace_ref, func_name, arguments)
    }

    /// evaluate a script
    pub fn eval(&self, script: Script) -> Result<QuickJsValueAdapter, JsError> {
        unsafe { Self::eval_ctx(self.context, script, None) }
    }

    pub fn eval_this(
        &self,
        script: Script,
        this: QuickJsValueAdapter,
    ) -> Result<QuickJsValueAdapter, JsError> {
        unsafe { Self::eval_ctx(self.context, script, Some(this)) }
    }

    /// # Safety
    /// when passing a context ptr please be sure that the corresponding QuickJsContext is still active
    pub unsafe fn eval_ctx(
        context: *mut q::JSContext,
        mut script: Script,
        this_opt: Option<QuickJsValueAdapter>,
    ) -> Result<QuickJsValueAdapter, JsError> {
        log::debug!("q_js_rt.eval file {}", script.get_path());

        script = QuickJsRuntimeAdapter::pre_process(script)?;

        let code_str = script.get_runnable_code();

        let filename_c = make_cstring(script.get_path())?;
        let code_c = make_cstring(code_str)?;

        let value_raw = match this_opt {
            None => q::JS_Eval(
                context,
                code_c.as_ptr(),
                code_str.len() as _,
                filename_c.as_ptr(),
                q::JS_EVAL_TYPE_GLOBAL as i32,
            ),
            Some(this) => q::JS_EvalThis(
                context,
                this.clone_value_incr_rc(),
                code_c.as_ptr(),
                code_str.len() as _,
                filename_c.as_ptr(),
                q::JS_EVAL_TYPE_GLOBAL as i32,
            ),
        };

        log::trace!("after eval, checking error");

        // check for error
        let ret = QuickJsValueAdapter::new(
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
    pub fn eval_module(&self, script: Script) -> Result<QuickJsValueAdapter, JsError> {
        unsafe { Self::eval_module_ctx(self.context, script) }
    }

    /// # Safety
    /// when passing a context ptr please be sure that the corresponding QuickJsContext is still active
    pub unsafe fn eval_module_ctx(
        context: *mut q::JSContext,
        mut script: Script,
    ) -> Result<QuickJsValueAdapter, JsError> {
        log::debug!("q_js_rt.eval_module file {}", script.get_path());

        script = QuickJsRuntimeAdapter::pre_process(script)?;

        let code_str = script.get_runnable_code();

        let filename_c = make_cstring(script.get_path())?;
        let code_c = make_cstring(code_str)?;

        let value_raw = q::JS_Eval(
            context,
            code_c.as_ptr(),
            code_str.len() as _,
            filename_c.as_ptr(),
            q::JS_EVAL_TYPE_MODULE as i32,
        );

        let ret = QuickJsValueAdapter::new(
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

    pub fn cache_object(&self, obj: QuickJsValueAdapter) -> i32 {
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

    pub fn consume_cached_obj(&self, id: i32) -> QuickJsValueAdapter {
        log::trace!("consume_cached_obj: id={}, thread={}", id, thread_id::get());
        let cache_map = &mut *self.object_cache.borrow_mut();
        cache_map.remove(&(id as usize))
    }

    pub fn with_cached_obj<C, R>(&self, id: i32, consumer: C) -> R
    where
        C: FnOnce(QuickJsValueAdapter) -> R,
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

        log::trace!("after drop QuickJSContext {}", self.id);
    }
}

impl QuickJsRealmAdapter {
    pub fn get_realm_id(&self) -> &str {
        self.id.as_str()
    }

    pub fn get_runtime_facade_inner(&self) -> Weak<QuickjsRuntimeFacadeInner> {
        QuickJsRuntimeAdapter::do_with(|rt| {
            Arc::downgrade(&rt.get_rti_ref().expect("Runtime was dropped"))
        })
    }

    pub fn get_script_or_module_name(&self) -> Result<String, JsError> {
        crate::quickjs_utils::get_script_or_module_name_q(self)
    }

    pub fn install_proxy(
        &self,
        proxy: JsProxy,
        add_global_var: bool,
    ) -> Result<QuickJsValueAdapter, JsError> {
        // create qjs proxy from proxy

        proxy.install(self, add_global_var)
    }

    pub fn instantiate_proxy_with_id(
        &self,
        namespace: &[&str],
        class_name: &str,
        instance_id: usize,
    ) -> Result<QuickJsValueAdapter, JsError> {
        // todo store proxies with slice/name as key?
        let cn = if namespace.is_empty() {
            class_name.to_string()
        } else {
            format!("{}.{}", namespace.join("."), class_name)
        };

        let proxy_map = self.proxy_registry.borrow();
        let proxy = proxy_map.get(cn.as_str()).expect("class not found");

        new_instance3(proxy, instance_id, self)
    }

    pub fn instantiate_proxy(
        &self,
        namespace: &[&str],
        class_name: &str,
        arguments: &[QuickJsValueAdapter],
    ) -> Result<(JsProxyInstanceId, QuickJsValueAdapter), JsError> {
        // todo store proxies with slice/name as key?
        let cn = if namespace.is_empty() {
            class_name.to_string()
        } else {
            format!("{}.{}", namespace.join("."), class_name)
        };

        let proxy_map = self.proxy_registry.borrow();
        let proxy = proxy_map.get(cn.as_str()).expect("class not found");

        let instance_info = new_instance(cn.as_str(), self)?;

        if let Some(constructor) = &proxy.constructor {
            // call constructor myself
            QuickJsRuntimeAdapter::do_with(|rt| constructor(rt, self, instance_info.0, arguments))?
        }

        Ok(instance_info)
    }

    pub fn dispatch_proxy_event(
        &self,
        namespace: &[&str],
        class_name: &str,
        proxy_instance_id: &usize,
        event_id: &str,
        event_obj: &QuickJsValueAdapter,
    ) -> Result<bool, JsError> {
        // todo store proxies with slice/name as key?
        let cn = if namespace.is_empty() {
            class_name.to_string()
        } else {
            format!("{}.{}", namespace.join("."), class_name)
        };

        let proxy_map = self.proxy_registry.borrow();
        let proxy = proxy_map.get(cn.as_str()).expect("class not found");

        dispatch_event(self, proxy, *proxy_instance_id, event_id, event_obj.clone())
    }

    pub fn dispatch_static_proxy_event(
        &self,
        namespace: &[&str],
        class_name: &str,
        event_id: &str,
        event_obj: &QuickJsValueAdapter,
    ) -> Result<bool, JsError> {
        // todo store proxies with slice/name as key?
        let cn = if namespace.is_empty() {
            class_name.to_string()
        } else {
            format!("{}.{}", namespace.join("."), class_name)
        };

        let proxy_map = self.proxy_registry.borrow();
        let proxy = proxy_map.get(cn.as_str()).expect("class not found");

        dispatch_static_event(
            self,
            proxy.get_class_name().as_str(),
            event_id,
            event_obj.clone(),
        )
    }

    pub fn install_function(
        &self,
        namespace: &[&str],
        name: &str,
        js_function: fn(
            &QuickJsRuntimeAdapter,
            &Self,
            &QuickJsValueAdapter,
            &[QuickJsValueAdapter],
        ) -> Result<QuickJsValueAdapter, JsError>,
        arg_count: u32,
    ) -> Result<(), JsError> {
        // todo namespace as slice?
        let ns = self.get_namespace(namespace)?;

        let func = functions::new_function_q(
            self,
            name,
            move |ctx, this, args| {
                QuickJsRuntimeAdapter::do_with(|rt| js_function(rt, ctx, this, args))
            },
            arg_count,
        )?;
        self.set_object_property(&ns, name, &func)?;
        Ok(())
    }

    pub fn install_closure<
        F: Fn(
                &QuickJsRuntimeAdapter,
                &Self,
                &QuickJsValueAdapter,
                &[QuickJsValueAdapter],
            ) -> Result<QuickJsValueAdapter, JsError>
            + 'static,
    >(
        &self,
        namespace: &[&str],
        name: &str,
        js_function: F,
        arg_count: u32,
    ) -> Result<(), JsError> {
        // todo namespace as slice?
        let ns = self.get_namespace(namespace)?;

        let func = functions::new_function_q(
            self,
            name,
            move |ctx, this, args| {
                QuickJsRuntimeAdapter::do_with(|rt| js_function(rt, ctx, this, args))
            },
            arg_count,
        )?;
        self.set_object_property(&ns, name, &func)?;
        Ok(())
    }

    pub fn get_global(&self) -> Result<QuickJsValueAdapter, JsError> {
        Ok(get_global_q(self))
    }

    pub fn get_namespace(&self, namespace: &[&str]) -> Result<QuickJsValueAdapter, JsError> {
        objects::get_namespace_q(self, namespace, true)
    }

    pub fn invoke_function_on_object_by_name(
        &self,
        this_obj: &QuickJsValueAdapter,
        method_name: &str,
        args: &[QuickJsValueAdapter],
    ) -> Result<QuickJsValueAdapter, JsError> {
        functions::invoke_member_function_q(self, this_obj, method_name, args)
    }

    pub fn invoke_function(
        &self,
        this_obj: Option<&QuickJsValueAdapter>,
        function_obj: &QuickJsValueAdapter,
        args: &[&QuickJsValueAdapter],
    ) -> Result<QuickJsValueAdapter, JsError> {
        functions::call_function_q_ref_args(self, function_obj, args, this_obj)
    }

    pub fn create_function<
        F: Fn(
                &Self,
                &QuickJsValueAdapter,
                &[QuickJsValueAdapter],
            ) -> Result<QuickJsValueAdapter, JsError>
            + 'static,
    >(
        &self,
        name: &str,
        js_function: F,
        arg_count: u32,
    ) -> Result<QuickJsValueAdapter, JsError> {
        functions::new_function_q(self, name, js_function, arg_count)
    }

    pub fn create_function_async<R, F>(
        &self,
        name: &str,
        js_function: F,
        arg_count: u32,
    ) -> Result<QuickJsValueAdapter, JsError>
    where
        Self: Sized + 'static,
        R: Future<Output = Result<JsValueFacade, JsError>> + Send + 'static,
        F: Fn(JsValueFacade, Vec<JsValueFacade>) -> R + 'static,
    {
        //
        self.create_function(
            name,
            move |realm, this, args| {
                let this_fac = realm.to_js_value_facade(this)?;
                let mut args_fac = vec![];
                for arg in args {
                    args_fac.push(realm.to_js_value_facade(arg)?);
                }
                let fut = js_function(this_fac, args_fac);
                realm.create_resolving_promise_async(fut, |realm, pres| {
                    //
                    realm.from_js_value_facade(pres)
                })
            },
            arg_count,
        )
    }

    pub fn create_error(
        &self,
        name: &str,
        message: &str,
        stack: &str,
    ) -> Result<QuickJsValueAdapter, JsError> {
        unsafe { errors::new_error(self.context, name, message, stack) }
    }

    pub fn delete_object_property(
        &self,
        object: &QuickJsValueAdapter,
        property_name: &str,
    ) -> Result<(), JsError> {
        // todo impl a real delete_prop
        objects::set_property_q(self, object, property_name, &new_null_ref())
    }

    pub fn set_object_property(
        &self,
        object: &QuickJsValueAdapter,
        property_name: &str,
        property: &QuickJsValueAdapter,
    ) -> Result<(), JsError> {
        objects::set_property_q(self, object, property_name, property)
    }

    pub fn get_object_property(
        &self,
        object: &QuickJsValueAdapter,
        property_name: &str,
    ) -> Result<QuickJsValueAdapter, JsError> {
        objects::get_property_q(self, object, property_name)
    }

    pub fn create_object(&self) -> Result<QuickJsValueAdapter, JsError> {
        objects::create_object_q(self)
    }

    pub fn construct_object(
        &self,
        constructor: &QuickJsValueAdapter,
        args: &[&QuickJsValueAdapter],
    ) -> Result<QuickJsValueAdapter, JsError> {
        // todo alter constructor method to accept slice
        unsafe { construct_object(self.context, constructor, args) }
    }

    pub fn get_object_properties(
        &self,
        object: &QuickJsValueAdapter,
    ) -> Result<Vec<String>, JsError> {
        let props = objects::get_own_property_names_q(self, object)?;
        let mut ret = vec![];
        for x in 0..props.len() {
            let prop = props.get_name(x)?;
            ret.push(prop);
        }
        Ok(ret)
    }

    pub fn traverse_object<F, R>(
        &self,
        object: &QuickJsValueAdapter,
        visitor: F,
    ) -> Result<Vec<R>, JsError>
    where
        F: Fn(&str, &QuickJsValueAdapter) -> Result<R, JsError>,
    {
        objects::traverse_properties_q(self, object, visitor)
    }

    pub fn traverse_object_mut<F>(
        &self,
        object: &QuickJsValueAdapter,
        visitor: F,
    ) -> Result<(), JsError>
    where
        F: FnMut(&str, &QuickJsValueAdapter) -> Result<(), JsError>,
    {
        objects::traverse_properties_q_mut(self, object, visitor)
    }

    pub fn get_array_element(
        &self,
        array: &QuickJsValueAdapter,
        index: u32,
    ) -> Result<QuickJsValueAdapter, JsError> {
        arrays::get_element_q(self, array, index)
    }

    /// push an element into an Array
    pub fn push_array_element(
        &self,
        array: &QuickJsValueAdapter,
        element: &QuickJsValueAdapter,
    ) -> Result<u32, JsError> {
        let push_func = self.get_object_property(array, "push")?;
        let res = self.invoke_function(Some(array), &push_func, &[element])?;
        Ok(res.to_i32() as u32)
    }

    pub fn set_array_element(
        &self,
        array: &QuickJsValueAdapter,
        index: u32,
        element: &QuickJsValueAdapter,
    ) -> Result<(), JsError> {
        arrays::set_element_q(self, array, index, element)
    }

    pub fn get_array_length(&self, array: &QuickJsValueAdapter) -> Result<u32, JsError> {
        arrays::get_length_q(self, array)
    }

    pub fn create_array(&self) -> Result<QuickJsValueAdapter, JsError> {
        arrays::create_array_q(self)
    }

    pub fn traverse_array<F, R>(
        &self,
        array: &QuickJsValueAdapter,
        visitor: F,
    ) -> Result<Vec<R>, JsError>
    where
        F: Fn(u32, &QuickJsValueAdapter) -> Result<R, JsError>,
    {
        // todo impl real traverse methods
        let mut ret = vec![];
        for x in 0..arrays::get_length_q(self, array)? {
            let val = arrays::get_element_q(self, array, x)?;
            ret.push(visitor(x, &val)?)
        }
        Ok(ret)
    }

    pub fn traverse_array_mut<F>(
        &self,
        array: &QuickJsValueAdapter,
        mut visitor: F,
    ) -> Result<(), JsError>
    where
        F: FnMut(u32, &QuickJsValueAdapter) -> Result<(), JsError>,
    {
        // todo impl real traverse methods
        for x in 0..arrays::get_length_q(self, array)? {
            let val = arrays::get_element_q(self, array, x)?;
            visitor(x, &val)?;
        }
        Ok(())
    }

    pub fn create_null(&self) -> Result<QuickJsValueAdapter, JsError> {
        Ok(crate::quickjs_utils::new_null_ref())
    }

    pub fn create_undefined(&self) -> Result<QuickJsValueAdapter, JsError> {
        Ok(crate::quickjs_utils::new_undefined_ref())
    }

    pub fn create_i32(&self, val: i32) -> Result<QuickJsValueAdapter, JsError> {
        Ok(from_i32(val))
    }

    pub fn create_string(&self, val: &str) -> Result<QuickJsValueAdapter, JsError> {
        from_string_q(self, val)
    }

    pub fn create_boolean(&self, val: bool) -> Result<QuickJsValueAdapter, JsError> {
        Ok(from_bool(val))
    }

    pub fn create_f64(&self, val: f64) -> Result<QuickJsValueAdapter, JsError> {
        Ok(from_f64(val))
    }

    pub fn create_promise(&self) -> Result<QuickJsPromiseAdapter, JsError> {
        crate::quickjs_utils::promises::new_promise_q(self)
    }

    pub fn add_promise_reactions(
        &self,
        promise: &QuickJsValueAdapter,
        then: Option<QuickJsValueAdapter>,
        catch: Option<QuickJsValueAdapter>,
        finally: Option<QuickJsValueAdapter>,
    ) -> Result<(), JsError> {
        crate::quickjs_utils::promises::add_promise_reactions_q(self, promise, then, catch, finally)
    }

    pub fn cache_promise(&self, promise_ref: QuickJsPromiseAdapter) -> usize {
        let map = &mut *self.promise_cache.borrow_mut();
        map.insert(promise_ref)
    }

    pub fn consume_cached_promise(&self, id: usize) -> Option<QuickJsPromiseAdapter> {
        let map = &mut *self.promise_cache.borrow_mut();
        map.remove_opt(&id)
    }

    pub fn dispose_cached_object(&self, id: i32) {
        let _ = self.consume_cached_obj(id);
    }

    pub fn with_cached_object<C, R>(&self, id: i32, consumer: C) -> R
    where
        C: FnOnce(&QuickJsValueAdapter) -> R,
    {
        self.with_cached_obj(id, |obj| consumer(&obj))
    }

    pub fn consume_cached_object(&self, id: i32) -> QuickJsValueAdapter {
        self.consume_cached_obj(id)
    }

    pub fn is_instance_of(
        &self,
        object: &QuickJsValueAdapter,
        constructor: &QuickJsValueAdapter,
    ) -> bool {
        objects::is_instance_of_q(self, object, constructor)
    }

    pub fn json_stringify(
        &self,
        object: &QuickJsValueAdapter,
        opt_space: Option<&str>,
    ) -> Result<String, JsError> {
        let opt_space_jsvr = match opt_space {
            None => None,
            Some(s) => Some(self.create_string(s)?),
        };
        let res = json::stringify_q(self, object, opt_space_jsvr);
        match res {
            Ok(jsvr) => jsvr.to_string(),
            Err(e) => Err(e),
        }
    }

    pub fn json_parse(&self, json_string: &str) -> Result<QuickJsValueAdapter, JsError> {
        json::parse_q(self, json_string)
    }

    pub fn create_typed_array_uint8(
        &self,
        buffer: Vec<u8>,
    ) -> Result<QuickJsValueAdapter, JsError> {
        new_uint8_array_q(self, buffer)
    }

    pub fn create_typed_array_uint8_copy(
        &self,
        buffer: &[u8],
    ) -> Result<QuickJsValueAdapter, JsError> {
        new_uint8_array_copy_q(self, buffer)
    }

    pub fn detach_typed_array_buffer(
        &self,
        array: &QuickJsValueAdapter,
    ) -> Result<Vec<u8>, JsError> {
        let abuf = get_array_buffer_q(self, array)?;
        detach_array_buffer_buffer_q(self, &abuf)
    }

    pub fn copy_typed_array_buffer(&self, array: &QuickJsValueAdapter) -> Result<Vec<u8>, JsError> {
        let abuf = get_array_buffer_q(self, array)?;
        get_array_buffer_buffer_copy_q(self, &abuf)
    }

    pub fn get_proxy_instance_info(
        &self,
        obj: &QuickJsValueAdapter,
    ) -> Result<(String, JsProxyInstanceId), JsError>
    where
        Self: Sized,
    {
        if let Some((p, i)) =
            crate::reflection::get_proxy_instance_proxy_and_instance_id_q(self, obj)
        {
            Ok((p.get_class_name(), i))
        } else {
            Err(JsError::new_str("not a proxy instance"))
        }
    }

    pub fn to_js_value_facade(
        &self,
        js_value: &QuickJsValueAdapter,
    ) -> Result<JsValueFacade, JsError>
    where
        Self: Sized + 'static,
    {
        let res: JsValueFacade = match js_value.get_js_type() {
            JsValueType::I32 => JsValueFacade::I32 {
                val: js_value.to_i32(),
            },
            JsValueType::F64 => JsValueFacade::F64 {
                val: js_value.to_f64(),
            },
            JsValueType::String => JsValueFacade::String {
                val: DefaultAtom::from(js_value.to_string()?),
            },
            JsValueType::Boolean => JsValueFacade::Boolean {
                val: js_value.to_bool(),
            },
            JsValueType::Object => {
                if js_value.is_typed_array() {
                    // todo TypedArray as JsValueType?
                    // passing a typedarray out of the worker thread is sketchy because you either copy the buffer like we do here, or you detach the buffer effectively destroying the jsvalue
                    // you should be better of optimizing this in native methods
                    JsValueFacade::TypedArray {
                        buffer: self.copy_typed_array_buffer(js_value)?,
                        array_type: TypedArrayType::Uint8,
                    }
                } else {
                    JsValueFacade::JsObject {
                        cached_object: CachedJsObjectRef::new(self, js_value.clone()),
                    }
                }
            }
            JsValueType::Function => JsValueFacade::JsFunction {
                cached_function: CachedJsFunctionRef {
                    cached_object: CachedJsObjectRef::new(self, js_value.clone()),
                },
            },
            JsValueType::BigInt => {
                todo!();
            }
            JsValueType::Promise => JsValueFacade::JsPromise {
                cached_promise: CachedJsPromiseRef {
                    cached_object: CachedJsObjectRef::new(self, js_value.clone()),
                },
            },
            JsValueType::Date => {
                todo!();
            }
            JsValueType::Null => JsValueFacade::Null,
            JsValueType::Undefined => JsValueFacade::Undefined,

            JsValueType::Array => JsValueFacade::JsArray {
                cached_array: CachedJsArrayRef {
                    cached_object: CachedJsObjectRef::new(self, js_value.clone()),
                },
            },
            JsValueType::Error => {
                let name = self.get_object_property(js_value, "name")?.to_string()?;
                let message = self.get_object_property(js_value, "message")?.to_string()?;
                let stack = self.get_object_property(js_value, "stack")?.to_string()?;

                #[cfg(feature = "typescript")]
                let stack = crate::typescript::unmap_stack_trace(stack.as_str());

                JsValueFacade::JsError {
                    val: JsError::new(name, message, stack),
                }
            }
        };
        Ok(res)
    }

    /// convert a JSValueFacade into a JSValueAdapter
    /// you need this to move values into the worker thread from a different thread (JSValueAdapter cannot leave the worker thread)
    #[allow(clippy::wrong_self_convention)]
    pub fn from_js_value_facade(
        &self,
        value_facade: JsValueFacade,
    ) -> Result<QuickJsValueAdapter, JsError>
    where
        Self: Sized + 'static,
    {
        match value_facade {
            JsValueFacade::I32 { val } => self.create_i32(val),
            JsValueFacade::F64 { val } => self.create_f64(val),
            JsValueFacade::String { val } => self.create_string(&val),
            JsValueFacade::Boolean { val } => self.create_boolean(val),
            JsValueFacade::JsObject { cached_object } => {
                // todo check realm (else copy? or error?)
                self.with_cached_object(cached_object.id, |obj| Ok(obj.clone()))
            }
            JsValueFacade::JsPromise { cached_promise } => {
                // todo check realm (else copy? or error?)
                self.with_cached_object(cached_promise.cached_object.id, |obj| Ok(obj.clone()))
            }
            JsValueFacade::JsArray { cached_array } => {
                // todo check realm (else copy? or error?)
                self.with_cached_object(cached_array.cached_object.id, |obj| Ok(obj.clone()))
            }
            JsValueFacade::JsFunction { cached_function } => {
                // todo check realm (else copy? or error?)
                self.with_cached_object(cached_function.cached_object.id, |obj| Ok(obj.clone()))
            }
            JsValueFacade::Object { val } => {
                let obj = self.create_object()?;
                for entry in val {
                    let prop = self.from_js_value_facade(entry.1)?;
                    self.set_object_property(&obj, entry.0.as_str(), &prop)?;
                }
                Ok(obj)
            }
            JsValueFacade::Array { val } => {
                let obj = self.create_array()?;
                for (x, entry) in val.into_iter().enumerate() {
                    let prop = self.from_js_value_facade(entry)?;
                    self.set_array_element(&obj, x as u32, &prop)?;
                }
                Ok(obj)
            }
            JsValueFacade::Promise { producer } => {
                let producer = &mut *producer.lock("from_js_value_facade").unwrap();
                if producer.is_some() {
                    self.create_resolving_promise_async(producer.take().unwrap(), |realm, jsvf| {
                        realm.from_js_value_facade(jsvf)
                    })
                } else {
                    self.create_null()
                }
            }
            JsValueFacade::Function {
                name,
                arg_count,
                func,
            } => {
                //

                self.create_function(
                    name.as_str(),
                    move |realm, _this, args| {
                        let mut esvf_args = vec![];
                        for arg in args {
                            esvf_args.push(realm.to_js_value_facade(arg)?);
                        }
                        let esvf_res: Result<JsValueFacade, JsError> = func(esvf_args.as_slice());

                        match esvf_res {
                            //
                            Ok(jsvf) => realm.from_js_value_facade(jsvf),
                            Err(err) => Err(err),
                        }
                    },
                    arg_count,
                )
            }
            JsValueFacade::Null => self.create_null(),
            JsValueFacade::Undefined => self.create_undefined(),
            JsValueFacade::JsError { val } => {
                self.create_error(val.get_name(), val.get_message(), val.get_stack())
            }
            JsValueFacade::ProxyInstance {
                instance_id,
                namespace,
                class_name,
            } => self.instantiate_proxy_with_id(namespace, class_name, instance_id),
            JsValueFacade::TypedArray { buffer, array_type } => match array_type {
                TypedArrayType::Uint8 => self.create_typed_array_uint8(buffer),
            },
            JsValueFacade::JsonStr { json } => self.json_parse(json.as_str()),
            JsValueFacade::SerdeValue { value } => self.serde_value_to_value_adapter(value),
        }
    }

    pub fn value_adapter_to_serde_value(
        &self,
        value_adapter: &QuickJsValueAdapter,
    ) -> Result<serde_json::Value, JsError> {
        match value_adapter.get_js_type() {
            JsValueType::I32 => Ok(Value::from(value_adapter.to_i32())),
            JsValueType::F64 => Ok(Value::from(value_adapter.to_f64())),
            JsValueType::String => Ok(Value::from(value_adapter.to_string()?)),
            JsValueType::Boolean => Ok(Value::from(value_adapter.to_bool())),
            JsValueType::Object => {
                let mut map: serde_json::Map<String, serde_json::Value> = serde_json::Map::new();
                self.traverse_object_mut(value_adapter, |k, v| {
                    map.insert(k.to_string(), self.value_adapter_to_serde_value(v)?);
                    Ok(())
                })?;
                let obj_val = serde_json::Value::Object(map);
                Ok(obj_val)
            }
            JsValueType::Array => {
                let mut arr: Vec<serde_json::Value> = vec![];
                self.traverse_array_mut(value_adapter, |_i, v| {
                    arr.push(self.value_adapter_to_serde_value(v)?);
                    Ok(())
                })?;
                let arr_val = serde_json::Value::Array(arr);
                Ok(arr_val)
            }
            JsValueType::Null => Ok(serde_json::Value::Null),
            JsValueType::Undefined => Ok(serde_json::Value::Null),
            JsValueType::Function => Ok(serde_json::Value::Null),
            JsValueType::BigInt => Ok(serde_json::Value::Null),
            JsValueType::Promise => Ok(serde_json::Value::Null),
            JsValueType::Date => Ok(serde_json::Value::Null),
            JsValueType::Error => Ok(serde_json::Value::Null),
        }
    }

    pub fn serde_value_to_value_adapter(
        &self,
        value: Value,
    ) -> Result<QuickJsValueAdapter, JsError> {
        match value {
            Value::Null => self.create_null(),
            Value::Bool(b) => self.create_boolean(b),
            Value::Number(n) => {
                if n.is_i64() {
                    let i = n.as_i64().unwrap();
                    if i <= i32::MAX as i64 {
                        self.create_i32(i as i32)
                    } else {
                        self.create_f64(i as f64)
                    }
                } else if n.is_u64() {
                    let i = n.as_u64().unwrap();
                    if i <= i32::MAX as u64 {
                        self.create_i32(i as i32)
                    } else {
                        self.create_f64(i as f64)
                    }
                } else {
                    // f64
                    let i = n.as_f64().unwrap();
                    self.create_f64(i)
                }
            }
            Value::String(s) => self.create_string(s.as_str()),
            Value::Array(a) => {
                let arr = self.create_array()?;
                for (x, aval) in (0_u32..).zip(a.into_iter()) {
                    let entry = self.serde_value_to_value_adapter(aval)?;
                    self.set_array_element(&arr, x, &entry)?;
                }
                Ok(arr)
            }
            Value::Object(o) => {
                let obj = self.create_object()?;
                for oval in o {
                    let entry = self.serde_value_to_value_adapter(oval.1)?;
                    self.set_object_property(&obj, oval.0.as_str(), &entry)?;
                }
                Ok(obj)
            }
        }
    }
    /// create a new Promise with a Future which will run async and then resolve or reject the promise
    /// the mapper is used to convert the result of the future into a JSValueAdapter
    pub fn create_resolving_promise_async<P, R: Send + 'static, M>(
        &self,
        producer: P,
        mapper: M,
    ) -> Result<QuickJsValueAdapter, JsError>
    where
        P: Future<Output = Result<R, JsError>> + Send + 'static,
        M: FnOnce(&QuickJsRealmAdapter, R) -> Result<QuickJsValueAdapter, JsError> + Send + 'static,
        Self: Sized + 'static,
    {
        new_resolving_promise_async(self, producer, mapper)
    }
    /// create a new Promise with a FnOnce producer which will run async and then resolve or reject the promise
    /// the mapper is used to convert the result of the future into a JSValueAdapter
    ///
    pub fn create_resolving_promise<P, R: Send + 'static, M>(
        &self,
        producer: P,
        mapper: M,
    ) -> Result<QuickJsValueAdapter, JsError>
    where
        P: FnOnce() -> Result<R, JsError> + Send + 'static,
        M: FnOnce(&QuickJsRealmAdapter, R) -> Result<QuickJsValueAdapter, JsError> + Send + 'static,
        Self: Sized + 'static,
    {
        new_resolving_promise(self, producer, mapper)
    }
}

#[cfg(test)]
pub mod tests {
    use crate::builder::QuickJsRuntimeBuilder;
    use crate::facades::tests::init_test_rt;
    use crate::jsutils::Script;
    use crate::quickjs_utils;
    use crate::quickjs_utils::primitives::to_i32;
    use crate::quickjs_utils::{functions, get_global_q, objects};

    #[test]
    fn test_eval() {
        let rt = init_test_rt();
        rt.exe_rt_task_in_event_loop(|q_js_rt| {
            let q_ctx = q_js_rt.get_main_realm();
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
