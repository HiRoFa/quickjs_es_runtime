use crate::eserror::EsError;
use crate::quickjs_utils;
use crate::quickjs_utils::functions::new_native_function;
use crate::quickjs_utils::{atoms, functions, get_global, objects, primitives};
use crate::quickjsruntime::{make_cstring, OwnedValueRef, QuickJsRuntime};
use libquickjs_sys as q;
use log::trace;
use std::cell::RefCell;
use std::collections::HashMap;
use std::ffi::CString;
use std::os::raw::c_char;

pub type ProxyConstructor = dyn Fn(Vec<OwnedValueRef>) -> i32 + 'static;
pub type ProxyFinalizer = dyn Fn(i32) + 'static;
pub type ProxyMethod = dyn Fn(&i32, Vec<OwnedValueRef>) -> OwnedValueRef + 'static;
pub type ProxyStaticMethod = dyn Fn(Vec<OwnedValueRef>) -> OwnedValueRef + 'static;

static CNAME: &str = "ProxyClass\0";

thread_local! {
    static EXOTIC: RefCell<q::JSClassExoticMethods> = RefCell::new(q::JSClassExoticMethods {
        get_own_property: None,
        get_own_property_names: None,
        delete_property: None,
        define_own_property: None,
        has_property: Some(proxy_instance_has_prop),
        get_property: Some(proxy_instance_get_prop),
        set_property: None,
    });
}
thread_local! {
    static CLASS_DEF: RefCell<q::JSClassDef> = {
    EXOTIC.with(|e_rc|{
        let exotic = &mut *e_rc.borrow_mut();
        RefCell::new(q::JSClassDef {
            class_name: CNAME.as_ptr() as *const c_char,
            finalizer: Some(finalizer),
            gc_mark: None,
            call: Some(js_class_call),
            exotic,
        })
    })
};
}

pub struct Proxy {
    name: Option<String>,
    constructor: Option<Box<ProxyConstructor>>,
    finalizer: Option<Box<ProxyFinalizer>>,
    methods: HashMap<String, Box<ProxyMethod>>,
    static_methods: HashMap<String, Box<ProxyStaticMethod>>,
}

impl Proxy {
    pub fn new() -> Self {
        Proxy {
            name: None,
            constructor: None,
            finalizer: None,
            methods: Default::default(),
            static_methods: Default::default(),
        }
    }
    pub fn name(mut self, name: &str) -> Self {
        self.name = Some(name.to_string());
        self
    }
    pub fn get_class_name(&self) -> &str {
        if let Some(n) = self.name.as_ref() {
            n.as_str()
        } else {
            "__nameless_class__"
        }
    }
    pub fn constructor<C>(mut self, constructor: C) -> Self
    where
        C: Fn(Vec<OwnedValueRef>) -> i32 + 'static,
    {
        self.constructor = Some(Box::new(constructor));
        self
    }
    pub fn finalizer<C>(mut self, finalizer: C) -> Self
    where
        C: Fn(i32) + 'static,
    {
        self.finalizer = Some(Box::new(finalizer));
        self
    }
    pub fn method<M>(mut self, name: &str, method: M) -> Self
    where
        M: Fn(&i32, Vec<OwnedValueRef>) -> OwnedValueRef + 'static,
    {
        self.methods.insert(name.to_string(), Box::new(method));
        self
    }
    pub fn static_method<M>(mut self, name: &str, method: M) -> Self
    where
        M: Fn(Vec<OwnedValueRef>) -> OwnedValueRef + 'static,
    {
        self.static_methods
            .insert(name.to_string(), Box::new(method));
        self
    }
    pub fn install(mut self, q_js_rt: &QuickJsRuntime) -> Result<(), EsError> {
        if self.name.is_none() {
            return Err(EsError::new_str("Proxy needs a name"));
        }

        self.install_js_class(q_js_rt);
        let class_ref = self.install_class_prop(q_js_rt)?;

        // these all set the same func with a different name, actual method will be gotten from proxy from registry
        //self.install_methods(q_js_rt, &class_ref)?;
        //self.install_getters_setters(q_js_rt, &class_ref)?;
        self.install_static_methods(q_js_rt, &class_ref)?;
        self.install_static_getters_setters(q_js_rt, &class_ref)?;

        // when we're done we store the proxy class in the registry so we can obtain method, getters, setters later
        self.install_move_to_registry();

        Ok(())
    }
    fn install_methods(
        &self,
        q_js_rt: &QuickJsRuntime,
        class_ref: &OwnedValueRef,
    ) -> Result<(), EsError> {
        //unimplemented!()

        log::trace!("install_methods {}", self.get_class_name());
        let pt_ref = objects::get_property(q_js_rt, class_ref, "prototype")?;

        for (name, method) in &self.methods {
            log::trace!("install_methods {} / {}", self.get_class_name(), name);

            let func_ref = functions::new_native_function(
                q_js_rt,
                name.as_str(),
                Some(proxy_instance_method),
                1,
                false,
            )?;
            objects::set_property(q_js_rt, &pt_ref, name.as_str(), func_ref)?;
        }

        Ok(())
    }
    fn install_getters_setters(
        &self,
        q_js_rt: &QuickJsRuntime,
        class_ref: &OwnedValueRef,
    ) -> Result<(), EsError> {
        //unimplemented!()

        log::trace!("install_getters_setters {}", self.get_class_name());

        Ok(())
    }
    fn install_static_methods(
        &self,
        q_js_rt: &QuickJsRuntime,
        class_ref: &OwnedValueRef,
    ) -> Result<(), EsError> {
        //unimplemented!()

        log::trace!("install_static_methods {}", self.get_class_name());

        Ok(())
    }
    fn install_static_getters_setters(
        &self,
        q_js_rt: &QuickJsRuntime,
        class_ref: &OwnedValueRef,
    ) -> Result<(), EsError> {
        //unimplemented!()

        log::trace!("install_static_getters_setters {}", self.get_class_name());

        Ok(())
    }
    fn install_move_to_registry(self) {
        let proxy = self;
        PROXY_REGISTRY.with(move |rc| {
            let reg_map = &mut *rc.borrow_mut();
            reg_map.insert(proxy.name.as_ref().unwrap().clone(), proxy);
        });
    }
    fn install_class_prop(&self, q_js_rt: &QuickJsRuntime) -> Result<OwnedValueRef, EsError> {
        let constructor_ref = new_native_function(
            q_js_rt,
            self.name.as_ref().unwrap().as_str(),
            Some(constructor),
            1,
            true,
        )
        .ok()
        .expect("shit failed yo");

        // todo impl namespace here

        let global_ref = get_global(q_js_rt);
        objects::set_property(
            q_js_rt,
            &global_ref,
            self.name.as_ref().unwrap().as_str(),
            constructor_ref,
        )
        .ok()
        .expect("could not set prop");

        log::trace!("set prop done");

        objects::get_property(q_js_rt, &global_ref, self.name.as_ref().unwrap().as_str())
    }
    fn install_js_class(&self, q_js_rt: &QuickJsRuntime) {
        let mut c_id: u32 = 0;
        let class_id: u32 = unsafe { q::JS_NewClassID(&mut c_id) };
        log::trace!("got class id {}", class_id);

        register_class_name(self.name.as_ref().unwrap().as_str(), class_id as i32);

        CLASS_DEF.with(|cd_rc| {
            let class_def = &*cd_rc.borrow();
            let res = unsafe { q::JS_NewClass(q_js_rt.runtime, class_id, class_def) };
            log::trace!("new class res {}", res);
        });
    }
}

thread_local! {
    static CLASSNAME_CLASSID_MAPPINGS: RefCell<HashMap<String, i32>> = RefCell::new(HashMap::new());
    static PROXY_REGISTRY: RefCell<HashMap<String, Proxy>> = RefCell::new(HashMap::new());
}

#[cfg(test)]
pub mod tests {
    use crate::esruntime::EsRuntime;
    use crate::esscript::EsScript;
    use crate::quickjs_utils::functions::new_native_function;
    use crate::quickjs_utils::reflection::{
        constructor, finalizer, js_class_call, register_class_name, Proxy,
    };
    use crate::quickjs_utils::{get_global, primitives};
    use crate::quickjsruntime::make_cstring;
    use libquickjs_sys as q;
    use std::sync::Arc;
    use std::time::Duration;

    /*

    JSClassExoticMethods {
        pub get_own_property: ::std::option::Option<
            unsafe extern "C" fn(
                ctx: *mut JSContext,
                desc: *mut JSPropertyDescriptor,
                obj: JSValue,
                prop: JSAtom,
            ) -> ::std::os::raw::c_int,
        >,
        pub get_own_property_names: ::std::option::Option<
            unsafe extern "C" fn(
                ctx: *mut JSContext,
                ptab: *mut *mut JSPropertyEnum,
                plen: *mut u32,
                obj: JSValue,
            ) -> ::std::os::raw::c_int,
        >,
        pub delete_property: ::std::option::Option<
            unsafe extern "C" fn(
                ctx: *mut JSContext,
                obj: JSValue,
                prop: JSAtom,
            ) -> ::std::os::raw::c_int,
        >,
        pub define_own_property: ::std::option::Option<
            unsafe extern "C" fn(
                ctx: *mut JSContext,
                this_obj: JSValue,
                prop: JSAtom,
                val: JSValue,
                getter: JSValue,
                setter: JSValue,
                flags: ::std::os::raw::c_int,
            ) -> ::std::os::raw::c_int,
        >,
        pub has_property: ::std::option::Option<
            unsafe extern "C" fn(
                ctx: *mut JSContext,
                obj: JSValue,
                atom: JSAtom,
            ) -> ::std::os::raw::c_int,
        >,
        pub get_property: ::std::option::Option<
            unsafe extern "C" fn(
                ctx: *mut JSContext,
                obj: JSValue,
                atom: JSAtom,
                receiver: JSValue,
            ) -> JSValue,
        >,
        pub set_property: ::std::option::Option<
            unsafe extern "C" fn(
                ctx: *mut JSContext,
                obj: JSValue,
                atom: JSAtom,
                value: JSValue,
                receiver: JSValue,
                flags: ::std::os::raw::c_int,
            ) -> ::std::os::raw::c_int,
        >,

            pub type JSClassFinalizer =
            ::std::option::Option<unsafe extern "C" fn(rt: *mut JSRuntime, val: JSValue)>;
        pub type JSClassGCMark = ::std::option::Option<
            unsafe extern "C" fn(rt: *mut JSRuntime, val: JSValue, mark_func: JS_MarkFunc),
        >;
        pub type JSClassCall = ::std::option::Option<
            unsafe extern "C" fn(
                ctx: *mut JSContext,
                func_obj: JSValue,
                this_val: JSValue,
                argc: ::std::os::raw::c_int,
                argv: *mut JSValue,
                flags: ::std::os::raw::c_int,
            ) -> JSValue,
        >;
        #[repr(C)]
        #[derive(Debug, Copy, Clone)]
        pub struct JSClassDef {
            pub class_name: *const ::std::os::raw::c_char,
            pub finalizer: JSClassFinalizer,
            pub gc_mark: JSClassGCMark,
            pub call: JSClassCall,
            pub exotic: *mut JSClassExoticMethods,
        }

             */

    #[test]
    pub fn test_proxy() {
        let rt: Arc<EsRuntime> = crate::esruntime::tests::TEST_ESRT.clone();
        let io = rt.add_to_event_queue_sync(|q_js_rt| {
            Proxy::new()
                .name("TestClass1")
                .constructor(|args| 123)
                .method("doIt", |obj_id, args| primitives::from_i32(531))
                .method("doIt2", |obj_id, args| primitives::from_i32(257))
                .static_method("sDoIt", |args| primitives::from_i32(9876))
                .static_method("sDoIt2", |args| primitives::from_i32(140))
                .install(q_js_rt)
                .ok()
                .expect("could not install proxy");
        });
        rt.eval_sync(EsScript::new(
            "test_proxy.es".to_string(),
            "let tc1 = new TestClass1(1, true, 'abc'); tc1.doIt(1, true, 'abc'); tc1 = null;"
                .to_string(),
        ))
        .ok()
        .expect("script failed");
    }
}

fn register_class_name(class_name: &str, class_id: i32) {
    CLASSNAME_CLASSID_MAPPINGS.with(|rc: &RefCell<HashMap<String, i32>>| {
        let mappings = &mut *rc.borrow_mut();
        mappings.insert(class_name.to_string(), class_id);
    });
}

fn resolve_class_id(class_name: &str) -> i32 {
    CLASSNAME_CLASSID_MAPPINGS.with(|rc: &RefCell<HashMap<String, i32>>| {
        let mappings = &*rc.borrow();
        *mappings.get(class_name).unwrap()
    })
}

unsafe extern "C" fn constructor(
    ctx: *mut q::JSContext,
    this_val: q::JSValue,
    _argc: ::std::os::raw::c_int,
    _argv: *mut q::JSValue,
) -> q::JSValue {
    log::trace!("constructor called, this_tag={}", this_val.tag);

    // this is the function we created earlier (the constructor)
    // so classname = this.name;
    let this_ref = OwnedValueRef::new_no_free(this_val);
    QuickJsRuntime::do_with(|q_js_rt| {
        let name_ref = objects::get_property(q_js_rt, &this_ref, "name")
            .ok()
            .expect("name get failed");
        let class_name = functions::call_to_string(q_js_rt, &name_ref)
            .ok()
            .expect("name.toString failed");

        log::trace!("classname={}", class_name);
        let class_id = resolve_class_id(class_name.as_str());

        log::trace!("constructor called, class_id={}", class_id);
        let class_val: q::JSValue = q::JS_NewObjectClass(ctx, class_id as i32);

        let class_val_ref = OwnedValueRef::new_no_free(class_val);

        if class_val_ref.is_exception() {
            return if let Some(e) = q_js_rt.get_exception() {
                panic!("could not create class:{} due to: {}", class_name, e);
            } else {
                panic!("could not create class:{}", class_name);
            };
        }

        objects::set_property2(
            q_js_rt,
            &class_val_ref,
            "_ES_INSTANCE_ID_",
            primitives::from_i32(2581),
            q::JS_PROP_NORMAL as i32, // not configurable, writable or enumerable
        )
        .ok()
        .expect("could not set instance id");

        objects::set_property2(
            q_js_rt,
            &class_val_ref,
            "_ES_CLASSNAME_",
            primitives::from_string(q_js_rt, class_name.as_str())
                .ok()
                .expect("from_string failed during constructor"),
            q::JS_PROP_NORMAL as i32, // not configurable, writable or enumerable
        )
        .ok()
        .expect("could not set class name");

        log::trace!("constructor done");

        class_val
    })
}

unsafe extern "C" fn finalizer(_rt: *mut q::JSRuntime, val: q::JSValue) {
    //todo
    log::trace!("finalizer called");
    OwnedValueRef::new(val);
}

unsafe extern "C" fn js_class_call(
    _ctx: *mut q::JSContext,
    _func_obj: q::JSValue,
    _this_val: q::JSValue,
    _argc: ::std::os::raw::c_int,
    _argv: *mut q::JSValue,
    _flags: ::std::os::raw::c_int,
) -> q::JSValue {
    log::trace!("js_class_call called");
    //todo
    crate::quickjs_utils::new_null()
}

unsafe extern "C" fn proxy_instance_get_prop(
    ctx: *mut q::JSContext,
    obj: q::JSValue,
    atom: q::JSAtom,
    receiver: q::JSValue,
) -> q::JSValue {
    trace!("proxy_instance_get_prop");

    let obj_ref = OwnedValueRef::new_no_free(obj);
    let receiver_ref = OwnedValueRef::new_no_free(receiver);

    QuickJsRuntime::do_with(|q_js_rt| {
        let prop_name = atoms::to_string(q_js_rt, &atom)
            .ok()
            .expect("could not get name");
        trace!("proxy_instance_get_prop: {}", prop_name);
        let class_name_ref = objects::get_property(q_js_rt, &obj_ref, "_ES_CLASSNAME_")
            .ok()
            .expect("could not get classname");
        let class_name = primitives::to_string(q_js_rt, &class_name_ref)
            .ok()
            .expect("could not to_string name");
        trace!("obj_ref.classname = {}", class_name);

        // see if we have a matching method
        PROXY_REGISTRY.with(|pr_rc| {
            let registry = &*pr_rc.borrow();
            let proxy = registry.get(&class_name).unwrap();

            if proxy.methods.contains_key(&prop_name) {
                trace!("found method for {}", prop_name);

                return functions::new_native_function(
                    q_js_rt,
                    prop_name.as_str(),
                    Some(proxy_instance_method),
                    1,
                    false,
                )
                .ok()
                .expect("could not create func")
                .consume_value();
            }

            // retur null if nothing was returned
            quickjs_utils::new_null()
        })
    })

    // get constructor name
    // get proxy
    // get method or getter or setter
    // return native func (cache those?)
}

unsafe extern "C" fn proxy_instance_has_prop(
    ctx: *mut q::JSContext,
    obj: q::JSValue,
    atom: q::JSAtom,
) -> ::std::os::raw::c_int {
    trace!("proxy_instance_has_prop");
    0
}

unsafe extern "C" fn proxy_instance_method(
    _ctx: *mut q::JSContext,
    this_val: q::JSValue,
    argc: ::std::os::raw::c_int,
    argv: *mut q::JSValue,
) -> q::JSValue {
    trace!("proxy_instance_method");
    QuickJsRuntime::do_with(|q_js_rt| {
        let arg_slice = std::slice::from_raw_parts(argv, argc as usize);
        let args_vec: Vec<OwnedValueRef> = arg_slice
            .iter()
            .map(|raw| OwnedValueRef::new_no_free(*raw))
            .collect::<Vec<_>>();

        for arg in &args_vec {
            trace!(
                "arg: {}",
                functions::call_to_string(q_js_rt, arg)
                    .ok()
                    .expect("could not tostring arg")
            )
        }

        OwnedValueRef::new_no_free(this_val);

        // return null if nothing was returned
        quickjs_utils::new_null()
    })
}
