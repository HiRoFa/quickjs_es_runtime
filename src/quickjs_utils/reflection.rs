use crate::eserror::EsError;
use crate::quickjs_utils;
use crate::quickjs_utils::functions::new_native_function;
use crate::quickjs_utils::primitives::from_string;
use crate::quickjs_utils::{atoms, functions, get_global, objects, primitives};
use crate::quickjsruntime::{OwnedValueRef, QuickJsRuntime};
use libquickjs_sys as q;
use log::trace;
use std::cell::RefCell;
use std::collections::HashMap;
use std::os::raw::{c_char, c_void};

pub type ProxyConstructor = dyn Fn(Vec<OwnedValueRef>) -> Result<usize, EsError> + 'static;
pub type ProxyFinalizer = dyn Fn(usize) + 'static;
pub type ProxyMethod =
    dyn Fn(&usize, Vec<OwnedValueRef>) -> Result<OwnedValueRef, EsError> + 'static;
pub type ProxyStaticMethod = dyn Fn(Vec<OwnedValueRef>) -> Result<OwnedValueRef, EsError> + 'static;
pub type ProxyStaticGetter = dyn Fn() -> Result<OwnedValueRef, EsError> + 'static;
pub type ProxyStaticSetter = dyn Fn(OwnedValueRef) -> Result<(), EsError> + 'static;
pub type ProxyGetter = dyn Fn(usize) -> Result<OwnedValueRef, EsError> + 'static;
pub type ProxySetter = dyn Fn(usize, OwnedValueRef) -> Result<(), EsError> + 'static;

static CNAME: &str = "ProxyInstanceClass\0";
static SCNAME: &str = "ProxyStaticClass\0";

thread_local! {
    static INSTANCE_ID_MAPPINGS: RefCell<HashMap<usize, Box<(usize, String)>>> = RefCell::new(HashMap::new());

    static PROXY_REGISTRY: RefCell<HashMap<String, Proxy>> = RefCell::new(HashMap::new());

    static PROXY_STATIC_EXOTIC: RefCell<q::JSClassExoticMethods> = RefCell::new(q::JSClassExoticMethods {
        get_own_property: None,
        get_own_property_names: None,
        delete_property: None,
        define_own_property: None,
        has_property: Some(proxy_static_has_prop),
        get_property: Some(proxy_static_get_prop),
        set_property: Some(proxy_static_set_prop),
    });

    static PROXY_INSTANCE_EXOTIC: RefCell<q::JSClassExoticMethods> = RefCell::new(q::JSClassExoticMethods {
        get_own_property: None,
        get_own_property_names: None,
        delete_property: None,
        define_own_property: None,
        has_property: Some(proxy_instance_has_prop),
        get_property: Some(proxy_instance_get_prop),
        set_property: Some(proxy_instance_set_prop),
    });

    static PROXY_STATIC_CLASS_DEF: RefCell<q::JSClassDef> = {
        PROXY_STATIC_EXOTIC.with(|e_rc|{
            let exotic = &mut *e_rc.borrow_mut();
            RefCell::new(q::JSClassDef {
                class_name: SCNAME.as_ptr() as *const c_char,
                finalizer: None,
                gc_mark: None,
                call: None,
                exotic,
            })
        })
    };

    static PROXY_INSTANCE_CLASS_DEF: RefCell<q::JSClassDef> = {
        PROXY_INSTANCE_EXOTIC.with(|e_rc|{
            let exotic = &mut *e_rc.borrow_mut();
            RefCell::new(q::JSClassDef {
                class_name: CNAME.as_ptr() as *const c_char,
                finalizer: Some(finalizer),
                gc_mark: None,
                call: None,
                exotic,
            })
        })
    };
    static PROXY_STATIC_CLASS_ID: RefCell<u32> = {
        let mut c_id: u32 = 0;
        let class_id: u32 = unsafe { q::JS_NewClassID(&mut c_id) };
        log::trace!("got static class id {}", class_id);

        PROXY_STATIC_CLASS_DEF.with(|cd_rc| {
            let class_def = &*cd_rc.borrow();
            QuickJsRuntime::do_with(|q_js_rt| {
                let res = unsafe { q::JS_NewClass(q_js_rt.runtime, class_id, class_def) };
                log::trace!("new static class res {}", res);
                // todo res should be 0 for ok
            });
        });

        RefCell::new(class_id)
    };
    static PROXY_INSTANCE_CLASS_ID: RefCell<u32> = {
        let mut c_id: u32 = 0;
        let class_id: u32 = unsafe { q::JS_NewClassID(&mut c_id) };
        log::trace!("got class id {}", class_id);

        PROXY_INSTANCE_CLASS_DEF.with(|cd_rc| {
            let class_def = &*cd_rc.borrow();
            QuickJsRuntime::do_with(|q_js_rt| {
                let res = unsafe { q::JS_NewClass(q_js_rt.runtime, class_id, class_def) };
                log::trace!("new class res {}", res);
                // todo res should be 0 for ok
            });
        });

        RefCell::new(class_id)
    };
}

pub struct Proxy {
    name: Option<String>,
    constructor: Option<Box<ProxyConstructor>>,
    finalizer: Option<Box<ProxyFinalizer>>,
    methods: HashMap<String, Box<ProxyMethod>>,
    static_methods: HashMap<String, Box<ProxyStaticMethod>>,
    static_getters_setters: HashMap<String, (Box<ProxyStaticGetter>, Box<ProxyStaticSetter>)>,
    getters_setters: HashMap<String, (Box<ProxyGetter>, Box<ProxySetter>)>,
}

impl Proxy {
    #[allow(dead_code)]
    pub fn new() -> Self {
        Proxy {
            name: None,
            constructor: None,
            finalizer: None,
            methods: Default::default(),
            static_methods: Default::default(),
            static_getters_setters: Default::default(),
            getters_setters: Default::default(),
        }
    }
    #[allow(dead_code)]
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
    #[allow(dead_code)]
    pub fn constructor<C>(mut self, constructor: C) -> Self
    where
        C: Fn(Vec<OwnedValueRef>) -> Result<usize, EsError> + 'static,
    {
        self.constructor = Some(Box::new(constructor));
        self
    }

    #[allow(dead_code)]
    pub fn finalizer<C>(mut self, finalizer: C) -> Self
    where
        C: Fn(usize) + 'static,
    {
        self.finalizer = Some(Box::new(finalizer));
        self
    }

    #[allow(dead_code)]
    pub fn method<M>(mut self, name: &str, method: M) -> Self
    where
        M: Fn(&usize, Vec<OwnedValueRef>) -> Result<OwnedValueRef, EsError> + 'static,
    {
        self.methods.insert(name.to_string(), Box::new(method));
        self
    }

    #[allow(dead_code)]
    pub fn static_method<M>(mut self, name: &str, method: M) -> Self
    where
        M: Fn(Vec<OwnedValueRef>) -> Result<OwnedValueRef, EsError> + 'static,
    {
        self.static_methods
            .insert(name.to_string(), Box::new(method));
        self
    }
    #[allow(dead_code)]
    pub fn static_getter_setter<G, S>(mut self, name: &str, getter: G, setter: S) -> Self
    where
        G: Fn() -> Result<OwnedValueRef, EsError> + 'static,
        S: Fn(OwnedValueRef) -> Result<(), EsError> + 'static,
    {
        self.static_getters_setters
            .insert(name.to_string(), (Box::new(getter), Box::new(setter)));
        self
    }

    #[allow(dead_code)]
    pub fn getter_setter<G, S>(mut self, name: &str, getter: G, setter: S) -> Self
    where
        G: Fn(usize) -> Result<OwnedValueRef, EsError> + 'static,
        S: Fn(usize, OwnedValueRef) -> Result<(), EsError> + 'static,
    {
        self.getters_setters
            .insert(name.to_string(), (Box::new(getter), Box::new(setter)));
        self
    }

    #[allow(dead_code)]
    pub fn install(self, q_js_rt: &QuickJsRuntime) -> Result<(), EsError> {
        if self.name.is_none() {
            return Err(EsError::new_str("Proxy needs a name"));
        }

        let _class_ref = self.install_class_prop(q_js_rt)?;

        self.install_move_to_registry();

        Ok(())
    }

    fn install_move_to_registry(self) {
        let proxy = self;
        PROXY_REGISTRY.with(move |rc| {
            let reg_map = &mut *rc.borrow_mut();
            reg_map.insert(proxy.name.as_ref().unwrap().clone(), proxy);
        });
    }
    fn install_class_prop(&self, q_js_rt: &QuickJsRuntime) -> Result<(), EsError> {
        // this creates a constructor function, adds it to the global scope and then makes an instance of the satic_proxy_class its prototype so we can add static_getters_setters and static_methods

        let static_class_id = PROXY_STATIC_CLASS_ID.with(|rc| *rc.borrow());

        let constructor_ref = new_native_function(
            q_js_rt,
            self.name.as_ref().unwrap().as_str(),
            Some(constructor),
            1,
            true,
        )?;

        let class_val: q::JSValue =
            unsafe { q::JS_NewObjectClass(q_js_rt.context, static_class_id as i32) };

        let class_val_ref = OwnedValueRef::new_no_free(class_val);

        if class_val_ref.is_exception() {
            return if let Some(e) = q_js_rt.get_exception() {
                Err(e)
            } else {
                Err(EsError::new_string(format!(
                    "could not create class:{}",
                    self.get_class_name()
                )))
            };
        }

        unsafe {
            q::JS_SetPrototype(
                q_js_rt.context,
                *constructor_ref.borrow_value(),
                *class_val_ref.borrow_value(),
            );
        }

        // todo impl namespace here

        objects::set_property2(
            q_js_rt,
            &constructor_ref,
            "name",
            primitives::from_string(q_js_rt, self.get_class_name())?,
            0,
        )?;

        let global_ref = get_global(q_js_rt);
        objects::set_property2(
            q_js_rt,
            &global_ref,
            self.name.as_ref().unwrap().as_str(),
            constructor_ref,
            0,
        )?;

        log::trace!("install_class_prop done");

        Ok(())
    }
}

#[cfg(test)]
pub mod tests {
    use crate::eserror::EsError;
    use crate::esruntime::EsRuntime;
    use crate::esscript::EsScript;
    use crate::quickjs_utils::reflection::Proxy;
    use crate::quickjs_utils::{functions, primitives};
    use crate::quickjsruntime::QuickJsRuntime;
    use log::trace;
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
        rt.add_to_event_queue_sync(|q_js_rt| {
            let res = Proxy::new()
                .name("TestClass1")
                .constructor(|_args| Ok(123))
                .method("doIt", |_obj_id, _args| Ok(primitives::from_i32(531)))
                .method("doIt2", |_obj_id, _args| Ok(primitives::from_i32(257)))
                .getter_setter(
                    "gVar",
                    |id| Ok(primitives::from_i32(147)),
                    |_id, _val| Ok(()),
                )
                .static_method("sDoIt", |_args| Ok(primitives::from_i32(9876)))
                .static_method("sDoIt2", |_args| Ok(primitives::from_i32(140)))
                .static_getter_setter(
                    "someThing",
                    || {
                        trace!("static getter called, returning 754");
                        Ok(primitives::from_i32(754))
                    },
                    |val| {
                        QuickJsRuntime::do_with(|q_js_rt| {
                            trace!(
                                "static setter called, set to {}",
                                functions::call_to_string(q_js_rt, &val)?
                            );
                            Ok(())
                        })
                    },
                )
                .finalizer(|id| log::trace!("run finalizer: {}", id))
                .install(q_js_rt);

            match res {
                Ok(_) => {}
                Err(e) => panic!("could nt install proxy: {}", e),
            }
        });

        let i2 = rt.eval_sync(EsScript::new(
            "test_proxy.es".to_string(),
            "let tc2 = new TestClass1(1, true, 'abc'); let r2 = tc2.doIt(1, true, 'abc'); tc2 = null; r2;"
                .to_string(),
        ))
        .ok()
        .expect("script failed");

        assert!(i2.is_i32());
        assert_eq!(i2.get_i32(), 531);

        let i = rt.eval_sync(EsScript::new(
            "test_proxy.es".to_string(),
            "let tc1 = new TestClass1(1, true, 'abc'); let r = tc1.doIt(1, true, 'abc'); tc1 = null; r;"
                .to_string(),
        ))
            .ok()
            .expect("script failed");

        assert!(i.is_i32());
        assert_eq!(i.get_i32(), 531);

        let i3_res = rt.eval_sync(EsScript::new(
            "test_proxy.es".to_string(),
            "TestClass1.sDoIt();".to_string(),
        ));

        if i3_res.is_err() {
            panic!("script failed: {}", i3_res.err().unwrap());
        }
        let i3 = i3_res.ok().unwrap();

        assert!(i3.is_i32());
        assert_eq!(i3.get_i32(), 9876);

        let i4 = rt
            .eval_sync(EsScript::new(
                "test_proxy.es".to_string(),
                "TestClass1.someThing = 1; TestClass1.someThing;".to_string(),
            ))
            .ok()
            .expect("script failed");

        assert!(i4.is_i32());
        assert_eq!(i4.get_i32(), 754);

        let i5 = rt
            .eval_sync(EsScript::new(
                "test_proxy.es".to_string(),
                "let tc5 = new TestClass1(); let r5 = tc5.gVar; tc5 = null; r5;".to_string(),
            ))
            .ok()
            .expect("script failed");

        assert!(i5.is_i32());
        assert_eq!(i5.get_i32(), 147);

        std::thread::sleep(Duration::from_secs(1));
    }
}

#[allow(dead_code)]
unsafe extern "C" fn constructor(
    ctx: *mut q::JSContext,
    this_val: q::JSValue,
    argc: ::std::os::raw::c_int,
    argv: *mut q::JSValue,
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

        PROXY_REGISTRY.with(|registry_rc| {
            let registry = &*registry_rc.borrow();
            if let Some(proxy) = registry.get(&class_name) {
                if let Some(constructor) = &proxy.constructor {
                    // construct

                    let arg_slice = std::slice::from_raw_parts(argv, argc as usize);
                    let args_vec: Vec<OwnedValueRef> = arg_slice
                        .iter()
                        .map(|raw| OwnedValueRef::new_no_free(*raw))
                        .collect::<Vec<_>>();

                    let instance_id_res = constructor(args_vec);
                    let instance_id = match instance_id_res {
                        Ok(id) => id,
                        Err(e) => panic!(e),
                    };

                    log::trace!("classname={}", class_name);
                    let class_id = PROXY_INSTANCE_CLASS_ID.with(|rc| *rc.borrow());

                    log::trace!("constructor called, class_id={}", class_id);
                    let class_val: q::JSValue = q::JS_NewObjectClass(ctx, class_id as i32);

                    let class_val_ref = OwnedValueRef::new_no_free(class_val);

                    if class_val_ref.is_exception() {
                        if let Some(e) = q_js_rt.get_exception() {
                            panic!("could not create class:{} due to: {}", class_name, e);
                        } else {
                            panic!("could not create class:{}", class_name);
                        };
                    }

                    INSTANCE_ID_MAPPINGS.with(|im_rc| {
                        let mappings = &mut *im_rc.borrow_mut();
                        assert!(!mappings.contains_key(&instance_id));

                        let mut bx = Box::new((instance_id, class_name.clone()));

                        let ibp: &mut (usize, String) = &mut *bx;
                        let info_ptr = ibp as *mut _ as *mut c_void;

                        mappings.insert(instance_id, bx);
                        q::JS_SetOpaque(*class_val_ref.borrow_value(), info_ptr);
                    });

                    log::trace!("constructor done");

                    class_val
                } else {
                    // todo report ex, not a constructor
                    quickjs_utils::new_null()
                }
            } else {
                // todo panic
                quickjs_utils::new_null()
            }
        })
    })
}

fn get_proxy_instance_info(val: &q::JSValue) -> &(usize, String) {
    let class_id = PROXY_INSTANCE_CLASS_ID.with(|rc| *rc.borrow());
    let info_ptr: *mut c_void = unsafe { q::JS_GetOpaque(*val, class_id) };
    let info: &mut (usize, String) = unsafe { &mut *(info_ptr as *mut (usize, String)) };
    info
}

#[allow(dead_code)]
unsafe extern "C" fn finalizer(_rt: *mut q::JSRuntime, val: q::JSValue) {
    //todo
    log::trace!("finalizer called");
    //QuickJsRuntime::do_with(|q_js_rt| {

    let info: &(usize, String) = get_proxy_instance_info(&val);
    trace!("finalize {}", info.0);

    PROXY_REGISTRY.with(|pr_rc| {
        let registry = &*pr_rc.borrow();
        let proxy = registry.get(&info.1).unwrap();
        if let Some(finalizer) = &proxy.finalizer {
            finalizer(info.0);
        }
        INSTANCE_ID_MAPPINGS.with(|ids_rc| {
            let id_map = &mut *ids_rc.borrow_mut();
            id_map.remove(&info.0);
        });
    });

    //});
}
#[allow(dead_code)]
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

#[allow(dead_code)]
unsafe extern "C" fn proxy_static_get_prop(
    _ctx: *mut q::JSContext,
    obj: q::JSValue,
    atom: q::JSAtom,
    receiver: q::JSValue,
) -> q::JSValue {
    // static proxy class, not an instance
    trace!("proxy_static_get_prop");

    let obj_ref = OwnedValueRef::new_no_free(obj);
    let receiver_ref = OwnedValueRef::new_no_free(receiver);
    QuickJsRuntime::do_with(|q_js_rt| {
        let proxy_name_ref = objects::get_property(q_js_rt, &receiver_ref, "name")
            .ok()
            .unwrap();
        let proxy_name = primitives::to_str(q_js_rt, &proxy_name_ref).ok().unwrap();
        trace!("proxy_static_get_prop: {}", proxy_name);

        let prop_name = atoms::to_string(q_js_rt, &atom)
            .ok()
            .expect("could not get name");
        trace!("proxy_static_get_prop: prop: {}", prop_name);

        PROXY_REGISTRY.with(|registry_rc| {
            let registry = &*registry_rc.borrow();
            if let Some(proxy) = registry.get(proxy_name) {
                if let Some(method) = proxy.static_methods.get(&prop_name) {
                    // todo, don't just return new val, add to receiver as prop
                    trace!("found method for {}", prop_name);

                    let function_data_ref = from_string(q_js_rt, prop_name.as_str())
                        .ok()
                        .expect("could not create function_data_ref");

                    functions::new_native_function_data(
                        q_js_rt,
                        Some(proxy_static_method),
                        1,
                        function_data_ref,
                    )
                    .ok()
                    .expect("could not create func")
                    .consume_value()
                } else if let Some(getter_setter) = proxy.static_getters_setters.get(&prop_name) {
                    // call the getter
                    let getter = &getter_setter.0;
                    let mut res: Result<OwnedValueRef, EsError> = getter();
                    match res {
                        Ok(mut g_val) => g_val.consume_value(),
                        Err(e) => {
                            // todo report ex
                            panic!("static getter produced error: {}", e);
                        }
                    }
                } else {
                    quickjs_utils::new_null()
                }
            } else {
                quickjs_utils::new_null()
            }
        })
    })
}

#[allow(dead_code)]
unsafe extern "C" fn proxy_instance_get_prop(
    _ctx: *mut q::JSContext,
    obj: q::JSValue,
    atom: q::JSAtom,
    receiver: q::JSValue,
) -> q::JSValue {
    trace!("proxy_instance_get_prop");

    // todo, impl getter_setter and THEN copy paste to static variant

    let _obj_ref = OwnedValueRef::new_no_free(obj);
    let _receiver_ref = OwnedValueRef::new_no_free(receiver);

    QuickJsRuntime::do_with(|q_js_rt| {
        let prop_name = atoms::to_string(q_js_rt, &atom)
            .ok()
            .expect("could not get name");
        trace!("proxy_instance_get_prop: {}", prop_name);

        let info = get_proxy_instance_info(&obj);

        let class_name = &info.1;

        trace!("obj_ref.classname = {}", class_name);

        // see if we have a matching method
        PROXY_REGISTRY.with(|pr_rc| {
            let registry = &*pr_rc.borrow();
            let proxy = registry.get(class_name).unwrap();
            if proxy.methods.contains_key(&prop_name) {
                // todo, don't just return new val, add to receiver as prop
                trace!("found method for {}", prop_name);

                let function_data_ref = from_string(q_js_rt, prop_name.as_str())
                    .ok()
                    .expect("could not create function_data_ref");

                functions::new_native_function_data(
                    q_js_rt,
                    Some(proxy_instance_method),
                    1,
                    function_data_ref,
                )
                .ok()
                .expect("could not create func")
                .consume_value()
            } else if let Some(getter_setter) = proxy.getters_setters.get(&prop_name) {
                // call the getter
                let getter = &getter_setter.0;
                let mut res: Result<OwnedValueRef, EsError> = getter(info.0);
                match res {
                    Ok(mut g_val) => g_val.consume_value(),
                    Err(e) => {
                        // todo report ex
                        panic!("static getter produced error: {}", e);
                    }
                }
            } else {
                // return null if nothing was returned
                quickjs_utils::new_null()
            }
        })
    })

    // get constructor name
    // get proxy
    // get method or getter or setter
    // return native func (cache those?)
}
#[allow(dead_code)]
unsafe extern "C" fn proxy_instance_has_prop(
    _ctx: *mut q::JSContext,
    _obj: q::JSValue,
    _atom: q::JSAtom,
) -> ::std::os::raw::c_int {
    trace!("proxy_instance_has_prop");
    0
}
#[allow(dead_code)]
unsafe extern "C" fn proxy_static_has_prop(
    _ctx: *mut q::JSContext,
    _obj: q::JSValue,
    _atom: q::JSAtom,
) -> ::std::os::raw::c_int {
    trace!("proxy_static_has_prop");
    0
}

unsafe extern "C" fn proxy_instance_method(
    _ctx: *mut q::JSContext,
    this_val: q::JSValue,
    argc: ::std::os::raw::c_int,
    argv: *mut q::JSValue,
    _magic: ::std::os::raw::c_int,
    func_data: *mut q::JSValue,
) -> q::JSValue {
    trace!("proxy_instance_method");
    QuickJsRuntime::do_with(|q_js_rt| {
        OwnedValueRef::new_no_free(this_val);

        let proxy_instance_info: &(usize, String) = get_proxy_instance_info(&this_val);

        let arg_slice = std::slice::from_raw_parts(argv, argc as usize);
        let args_vec: Vec<OwnedValueRef> = arg_slice
            .iter()
            .map(|raw| OwnedValueRef::new_no_free(*raw))
            .collect::<Vec<_>>();

        let func_name_ref = OwnedValueRef::new_no_free(*func_data);
        let func_name = primitives::to_str(q_js_rt, &func_name_ref)
            .ok()
            .expect("could not to_string func_name_ref");

        trace!("proxy_instance_method: {}", func_name);

        PROXY_REGISTRY.with(|registry_rc| {
            let registry = &*registry_rc.borrow();
            let proxy = registry.get(proxy_instance_info.1.as_str()).unwrap();
            if let Some(method) = proxy.methods.get(func_name) {
                // todo report ex
                let mut m_res = method(&proxy_instance_info.0, args_vec)
                    .ok()
                    .expect("method failed");

                m_res.consume_value()
            } else {
                // return null if nothing was returned
                quickjs_utils::new_null()
            }
        })
    })
}

unsafe extern "C" fn proxy_static_getter(
    _ctx: *mut q::JSContext,
    this_val: q::JSValue,
    argc: ::std::os::raw::c_int,
    argv: *mut q::JSValue,
    _magic: ::std::os::raw::c_int,
    func_data: *mut q::JSValue,
) -> q::JSValue {
    trace!("proxy_static_getter called");

    quickjs_utils::new_null()
}

unsafe extern "C" fn proxy_static_setter(
    _ctx: *mut q::JSContext,
    this_val: q::JSValue,
    argc: ::std::os::raw::c_int,
    argv: *mut q::JSValue,
    _magic: ::std::os::raw::c_int,
    func_data: *mut q::JSValue,
) -> q::JSValue {
    trace!("proxy_static_setter called");

    quickjs_utils::new_null()
}

unsafe extern "C" fn proxy_static_method(
    _ctx: *mut q::JSContext,
    this_val: q::JSValue,
    argc: ::std::os::raw::c_int,
    argv: *mut q::JSValue,
    _magic: ::std::os::raw::c_int,
    func_data: *mut q::JSValue,
) -> q::JSValue {
    trace!("proxy_static_method");
    QuickJsRuntime::do_with(|q_js_rt| {
        let this_ref = OwnedValueRef::new_no_free(this_val);

        let proxy_name_ref = objects::get_property(q_js_rt, &this_ref, "name")
            .ok()
            .unwrap();
        let proxy_name = primitives::to_str(q_js_rt, &proxy_name_ref)
            .ok()
            .expect("could not to_string classname");

        let arg_slice = std::slice::from_raw_parts(argv, argc as usize);
        let args_vec: Vec<OwnedValueRef> = arg_slice
            .iter()
            .map(|raw| OwnedValueRef::new_no_free(*raw))
            .collect::<Vec<_>>();

        let func_name_ref = OwnedValueRef::new_no_free(*func_data);
        let func_name = primitives::to_str(q_js_rt, &func_name_ref)
            .ok()
            .expect("could not to_string func_name_ref");

        trace!("proxy_instance_method: {}", func_name);

        PROXY_REGISTRY.with(|registry_rc| {
            let registry = &*registry_rc.borrow();
            let proxy = registry.get(proxy_name).unwrap();
            if let Some(method) = proxy.static_methods.get(func_name) {
                let mut m_res: OwnedValueRef = method(args_vec).ok().expect("method failed");
                m_res.consume_value()
            } else {
                // return null if nothing was returned
                quickjs_utils::new_null()
            }
        })
    })
}

unsafe extern "C" fn proxy_static_set_prop(
    ctx: *mut q::JSContext,
    obj: q::JSValue,
    atom: q::JSAtom,
    value: q::JSValue,
    receiver: q::JSValue,
    flags: ::std::os::raw::c_int,
) -> ::std::os::raw::c_int {
    trace!("proxy_static_set_prop");
    0
}

unsafe extern "C" fn proxy_instance_set_prop(
    ctx: *mut q::JSContext,
    obj: q::JSValue,
    atom: q::JSAtom,
    value: q::JSValue,
    receiver: q::JSValue,
    flags: ::std::os::raw::c_int,
) -> ::std::os::raw::c_int {
    trace!("proxy_instance_set_prop");
    0
}
