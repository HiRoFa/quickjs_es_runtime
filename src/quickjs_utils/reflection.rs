use crate::eserror::EsError;
use crate::quickjs_utils;
use crate::quickjs_utils::functions::new_native_function;
use crate::quickjs_utils::primitives::from_string;
use crate::quickjs_utils::{atoms, errors, functions, get_global, objects, primitives};
use crate::quickjsruntime::QuickJsRuntime;
use crate::valueref::JSValueRef;
use libquickjs_sys as q;
use log::trace;
use std::cell::RefCell;
use std::collections::HashMap;
use std::os::raw::{c_char, c_void};

pub type ProxyConstructor = dyn Fn(Vec<JSValueRef>) -> Result<usize, EsError> + 'static;
pub type ProxyFinalizer = dyn Fn(usize) + 'static;
pub type ProxyMethod = dyn Fn(&usize, Vec<JSValueRef>) -> Result<JSValueRef, EsError> + 'static;
pub type ProxyStaticMethod = dyn Fn(Vec<JSValueRef>) -> Result<JSValueRef, EsError> + 'static;
pub type ProxyStaticGetter = dyn Fn() -> Result<JSValueRef, EsError> + 'static;
pub type ProxyStaticSetter = dyn Fn(JSValueRef) -> Result<(), EsError> + 'static;
pub type ProxyGetter = dyn Fn(usize) -> Result<JSValueRef, EsError> + 'static;
pub type ProxySetter = dyn Fn(usize, JSValueRef) -> Result<(), EsError> + 'static;

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

impl Default for quickjs_utils::reflection::Proxy {
    fn default() -> Self {
        Self::new()
    }
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
        C: Fn(Vec<JSValueRef>) -> Result<usize, EsError> + 'static,
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
        M: Fn(&usize, Vec<JSValueRef>) -> Result<JSValueRef, EsError> + 'static,
    {
        self.methods.insert(name.to_string(), Box::new(method));
        self
    }

    #[allow(dead_code)]
    pub fn static_method<M>(mut self, name: &str, method: M) -> Self
    where
        M: Fn(Vec<JSValueRef>) -> Result<JSValueRef, EsError> + 'static,
    {
        self.static_methods
            .insert(name.to_string(), Box::new(method));
        self
    }
    #[allow(dead_code)]
    pub fn static_getter_setter<G, S>(mut self, name: &str, getter: G, setter: S) -> Self
    where
        G: Fn() -> Result<JSValueRef, EsError> + 'static,
        S: Fn(JSValueRef) -> Result<(), EsError> + 'static,
    {
        self.static_getters_setters
            .insert(name.to_string(), (Box::new(getter), Box::new(setter)));
        self
    }

    #[allow(dead_code)]
    pub fn getter_setter<G, S>(mut self, name: &str, getter: G, setter: S) -> Self
    where
        G: Fn(usize) -> Result<JSValueRef, EsError> + 'static,
        S: Fn(usize, JSValueRef) -> Result<(), EsError> + 'static,
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

        let class_val_ref = JSValueRef::new(
            class_val,
            false,
            true,
            "reflection::Proxy::install_class_prop class_val",
        );

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
            let _res = q::JS_SetPrototype(
                q_js_rt.context,
                *constructor_ref.borrow_value(),
                class_val_ref.consume_value_no_decr_rc(),
            );
            /*if res != 0 {
                return if let Some(err) = q_js_rt.get_exception() {
                    Err(err)
                } else {
                    Err(EsError::new_str("could not set class proto"))
                };
            }*/
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
    use hirofa_utils::auto_id_map::AutoIdMap;
    use log::trace;
    use std::cell::RefCell;
    use std::sync::Arc;
    use std::time::Duration;

    thread_local! {
        static TEST_INSTANCES: RefCell<AutoIdMap<String>> = RefCell::new(AutoIdMap::new())
    }

    #[test]
    pub fn test_proxy() {
        log::info!("> test_proxy");

        let rt: Arc<EsRuntime> = crate::esruntime::tests::TEST_ESRT.clone();
        rt.add_to_event_queue_sync(|q_js_rt| {
            let res = Proxy::new()
                .name("TestClass1")
                .constructor(|_args| {
                    let id = TEST_INSTANCES.with(|rc| {
                        let map = &mut *rc.borrow_mut();
                        map.insert("hi".to_string())
                    });
                    Ok(id)
                })
                .method("doIt", |_obj_id, _args| Ok(primitives::from_i32(531)))
                .method("doIt2", |_obj_id, _args| Err(EsError::new_str("aaargh")))
                .getter_setter(
                    "gVar",
                    |_id| Ok(primitives::from_i32(147)),
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
                .finalizer(|id| {
                    TEST_INSTANCES.with(|rc| {
                        let map = &mut *rc.borrow_mut();
                        let _ = map.remove(&id);
                    });
                    log::trace!("ran finalizer: {}", id);
                })
                .install(q_js_rt);

            match res {
                Ok(_) => {}
                Err(e) => panic!("could nt install proxy: {}", e),
            }
        });

        let i2_res = rt.eval_sync(EsScript::new(
            "test_proxy.es",
            "let tc2 = new TestClass1(1, true, 'abc'); let r2 = tc2.doIt(1, true, 'abc'); console.log('< setting tc2 to null'); tc2 = null; console.log('> setting tc2 to null'); r2;"
                ,
        ));
        log::debug!("test_proxy.es done, ok = {}", i2_res.is_ok());
        match i2_res {
            Ok(i2) => {
                assert!(i2.is_i32());
                assert_eq!(i2.get_i32(), 531);
            }
            Err(e) => {
                log::error!("test_proxy.es failed with: {}", e);
                panic!("test_proxy.es failed");
            }
        }

        let i = rt.eval_sync(EsScript::new(
            "test_proxy2.es",
            "let tc1 = new TestClass1(1, true, 'abc'); let r = tc1.doIt(1, true, 'abc'); r = tc1.doIt(1, true, 'abc'); tc1 = null; r;"
        ))
            .ok()
            .expect("script failed");

        assert!(i.is_i32());
        assert_eq!(i.get_i32(), 531);

        let i3_res = rt.eval_sync(EsScript::new("test_proxy.es", "TestClass1.sDoIt();"));

        if i3_res.is_err() {
            panic!("script failed: {}", i3_res.err().unwrap());
        }
        let i3 = i3_res.ok().unwrap();

        assert!(i3.is_i32());
        assert_eq!(i3.get_i32(), 9876);

        let i4 = rt
            .eval_sync(EsScript::new(
                "test_proxy.es",
                "TestClass1.someThing = 1; TestClass1.someThing;",
            ))
            .ok()
            .expect("script failed");

        assert!(i4.is_i32());
        assert_eq!(i4.get_i32(), 754);

        let i5 = rt
            .eval_sync(EsScript::new(
                "test_proxy.es",
                "let tc5 = new TestClass1(); let r5 = tc5.gVar; tc5 = null; r5;",
            ))
            .ok()
            .expect("script failed");

        assert!(i5.is_i32());
        assert_eq!(i5.get_i32(), 147);

        let i6_res = rt.eval_sync(EsScript::new(
            "test_proxy.es",
            "let tc6 = new TestClass1(); let r6 = tc6.doIt2(); tc6 = null; r6;",
        ));
        assert!(i6_res.is_err());
        let e = i6_res.err().unwrap();
        let e_msg = e.get_message();
        assert_eq!(e_msg, "proxy_instance_method failed: aaargh");

        rt.gc_sync();

        std::thread::sleep(Duration::from_secs(1));

        log::info!("< test_proxy");
    }
}

pub fn new_instance2(
    proxy: &Proxy,
    instance_id: usize,
    q_js_rt: &QuickJsRuntime,
) -> Result<JSValueRef, EsError> {
    let ctx = q_js_rt.context;
    let class_id = PROXY_INSTANCE_CLASS_ID.with(|rc| *rc.borrow());

    let class_val: q::JSValue = unsafe { q::JS_NewObjectClass(ctx, class_id as i32) };

    let class_name = proxy.get_class_name();

    let class_val_ref = JSValueRef::new(
        class_val,
        false,
        true,
        format!("reflection::Proxy; cn={}", class_name).as_str(),
    );

    if class_val_ref.is_exception() {
        return if let Some(e) = q_js_rt.get_exception() {
            Err(EsError::new_string(format!(
                "could not create class:{} due to: {}",
                class_name, e
            )))
        } else {
            Err(EsError::new_string(format!(
                "could not create class:{}",
                class_name
            )))
        };
    }

    INSTANCE_ID_MAPPINGS.with(|im_rc| {
        let mappings = &mut *im_rc.borrow_mut();
        assert!(!mappings.contains_key(&instance_id));

        let mut bx = Box::new((instance_id, proxy.get_class_name().to_string()));

        let ibp: &mut (usize, String) = &mut *bx;
        let info_ptr = ibp as *mut _ as *mut c_void;

        mappings.insert(instance_id, bx);
        unsafe { q::JS_SetOpaque(*class_val_ref.borrow_value(), info_ptr) };
    });

    Ok(class_val_ref)
}

pub fn new_instance(
    class_name: &str,
    instance_id: usize,
    q_js_rt: &QuickJsRuntime,
) -> Result<JSValueRef, EsError> {
    // todo

    PROXY_REGISTRY.with(|registry_rc| {
        let registry = &*registry_rc.borrow();
        if let Some(proxy) = registry.get(class_name) {
            // construct

            new_instance2(proxy, instance_id, q_js_rt)
        } else {
            Err(EsError::new_str("no such proxy"))
        }
    })
}

#[allow(dead_code)]
unsafe extern "C" fn constructor(
    _ctx: *mut q::JSContext,
    this_val: q::JSValue,
    argc: ::std::os::raw::c_int,
    argv: *mut q::JSValue,
) -> q::JSValue {
    log::trace!("constructor called, this_tag={}", this_val.tag);

    // this is the function we created earlier (the constructor)
    // so classname = this.name;
    let this_ref = JSValueRef::new(this_val, false, false, "reflection::constructor this_val");
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
                    let args_vec: Vec<JSValueRef> = arg_slice
                        .iter()
                        .map(|raw| {
                            JSValueRef::new(*raw, false, false, "reflection::constructor arg")
                        })
                        .collect::<Vec<_>>();

                    let instance_id_res = constructor(args_vec);

                    match instance_id_res {
                        Ok(instance_id) => {
                            let instance_ref_res = new_instance2(proxy, instance_id, q_js_rt);

                            match instance_ref_res {
                                Ok(instance_ref) => instance_ref.consume_value_no_decr_rc(),
                                Err(e) => q_js_rt.report_ex(
                                    format!(
                                        "could not create proxy instance for {} due to {}",
                                        class_name, e
                                    )
                                    .as_str(),
                                ),
                            }
                        }
                        Err(es_err) => q_js_rt.report_ex(
                            format!("constructor for {} failed with {}", class_name, es_err)
                                .as_str(),
                        ),
                    }
                } else {
                    q_js_rt.report_ex("not a constructor")
                }
            } else {
                q_js_rt.report_ex("no such proxy")
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

    let info: &(usize, String) = get_proxy_instance_info(&val);
    trace!("finalize {}", info.0);

    PROXY_REGISTRY.with(|pr_rc| {
        let registry = &*pr_rc.borrow();
        let proxy = registry.get(&info.1).unwrap();
        if let Some(finalizer) = &proxy.finalizer {
            log::trace!("calling Proxy's finalizer");
            finalizer(info.0);
            log::trace!("after calling Proxy's finalizer");
        }
        INSTANCE_ID_MAPPINGS.with(|ids_rc| {
            log::trace!("reflection::finalizer: remove from INSTANCE_ID_MAPPINGS");
            let id_map = &mut *ids_rc.borrow_mut();
            let _ = id_map.remove(&info.0).expect("no such id to finalize");
            log::trace!("reflection::finalizer: remove from INSTANCE_ID_MAPPINGS -> done");
        });
        log::trace!("reflection::finalizer: 2");
    });
    log::trace!("reflection::finalizer: 3, exit");
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

    let _obj_ref = JSValueRef::new(obj, false, false, "reflection::proxy_static_get_prop obj");
    let receiver_ref = JSValueRef::new(
        receiver,
        false,
        false,
        "reflection::proxy_static_get_prop receiver",
    );
    QuickJsRuntime::do_with(|q_js_rt| {
        let proxy_name_ref = objects::get_property(q_js_rt, &receiver_ref, "name")
            .ok()
            .unwrap();
        let proxy_name = primitives::to_str(q_js_rt, &proxy_name_ref).ok().unwrap();
        trace!("proxy_static_get_prop: {}", proxy_name);

        let prop_name = atoms::to_string2(q_js_rt, &atom)
            .ok()
            .expect("could not get name");
        trace!("proxy_static_get_prop: prop: {}", prop_name);

        PROXY_REGISTRY.with(|registry_rc| {
            let registry = &*registry_rc.borrow();
            if let Some(proxy) = registry.get(proxy_name) {
                if proxy.static_methods.contains_key(&prop_name) {
                    trace!("found method for {}", prop_name);

                    let function_data_ref = from_string(q_js_rt, prop_name.as_str())
                        .ok()
                        .expect("could not create function_data_ref");

                    let func_ref = functions::new_native_function_data(
                        q_js_rt,
                        Some(proxy_static_method),
                        1,
                        function_data_ref,
                    )
                    .ok()
                    .expect("could not create func");

                    objects::set_property(
                        q_js_rt,
                        &receiver_ref,
                        prop_name.as_str(),
                        func_ref.clone(),
                    )
                    .ok()
                    .expect("set_property 9656738 failed");

                    func_ref.consume_value_no_decr_rc()
                } else if let Some(getter_setter) = proxy.static_getters_setters.get(&prop_name) {
                    // call the getter
                    let getter = &getter_setter.0;
                    let res: Result<JSValueRef, EsError> = getter();
                    match res {
                        Ok(g_val) => g_val.consume_value_no_decr_rc(),
                        Err(e) => {
                            let es = format!("proxy_static_get_prop failed: {}", e);
                            q_js_rt.report_ex(es.as_str())
                        }
                    }
                } else {
                    quickjs_utils::new_null()
                }
            } else {
                q_js_rt.report_ex("proxy class not found")
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

    let _obj_ref = JSValueRef::new(obj, false, false, "reflection::proxy_instance_get_prop obj");
    let receiver_ref = JSValueRef::new(
        receiver,
        false,
        false,
        "reflection::proxy_instance_get_prop receiver",
    );

    QuickJsRuntime::do_with(|q_js_rt| {
        let prop_name = atoms::to_string2(q_js_rt, &atom)
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

                let func_ref = functions::new_native_function_data(
                    q_js_rt,
                    Some(proxy_instance_method),
                    1,
                    function_data_ref,
                )
                .ok()
                .expect("could not create func");

                objects::set_property(q_js_rt, &receiver_ref, prop_name.as_str(), func_ref.clone())
                    .ok()
                    .expect("set_property 96385 failed");

                func_ref.consume_value_no_decr_rc()
            } else if let Some(getter_setter) = proxy.getters_setters.get(&prop_name) {
                // call the getter
                let getter = &getter_setter.0;
                let res: Result<JSValueRef, EsError> = getter(info.0);
                match res {
                    Ok(g_val) => g_val.consume_value_no_decr_rc(),
                    Err(e) => {
                        let err = format!("proxy_instance_get_prop failed: {}", e);
                        q_js_rt.report_ex(err.as_str())
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
        //let _this_ref = JSValueRef::new(this_val, "reflection::proxy_instance_method this_val");

        let proxy_instance_info: &(usize, String) = get_proxy_instance_info(&this_val);

        let arg_slice = std::slice::from_raw_parts(argv, argc as usize);
        let args_vec: Vec<JSValueRef> = arg_slice
            .iter()
            .map(|raw| JSValueRef::new(*raw, false, false, "reflection::proxy_instance_method arg"))
            .collect::<Vec<_>>();

        let func_name_ref = JSValueRef::new(
            *func_data,
            false,
            false,
            "reflection::proxy_instance_method func_data",
        );
        let func_name = primitives::to_str(q_js_rt, &func_name_ref)
            .ok()
            .expect("could not to_string func_name_ref");

        trace!("proxy_instance_method: {}", func_name);

        PROXY_REGISTRY.with(|registry_rc| {
            let registry = &*registry_rc.borrow();
            let proxy = registry.get(proxy_instance_info.1.as_str()).unwrap();
            if let Some(method) = proxy.methods.get(func_name) {
                // todo report ex
                let m_res: Result<JSValueRef, EsError> = method(&proxy_instance_info.0, args_vec);

                match m_res {
                    Ok(m_res_ref) => m_res_ref.consume_value_no_decr_rc(),
                    Err(e) => {
                        let msg = format!("proxy_instance_method failed: {}", e.get_message());
                        let err =
                            errors::new_error(q_js_rt, e.get_name(), msg.as_str(), e.get_stack())
                                .ok()
                                .expect("create error failed");
                        errors::throw(q_js_rt, err)
                    }
                }
            } else {
                // return null if nothing was returned
                quickjs_utils::new_null()
            }
        })
    })
}

#[allow(dead_code)]
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
        let this_ref = JSValueRef::new(
            this_val,
            false,
            false,
            "reflection::proxy_static_method this_val",
        );

        let proxy_name_ref = objects::get_property(q_js_rt, &this_ref, "name")
            .ok()
            .unwrap();
        let proxy_name = primitives::to_str(q_js_rt, &proxy_name_ref)
            .ok()
            .expect("could not to_string classname");

        let arg_slice = std::slice::from_raw_parts(argv, argc as usize);
        let args_vec: Vec<JSValueRef> = arg_slice
            .iter()
            .map(|raw| JSValueRef::new(*raw, false, false, "reflection::proxy_static_method arg"))
            .collect::<Vec<_>>();

        let func_name_ref = JSValueRef::new(
            *func_data,
            false,
            false,
            "reflection::proxy_static_method func_data",
        );
        let func_name = primitives::to_str(q_js_rt, &func_name_ref)
            .ok()
            .expect("could not to_string func_name_ref");

        trace!("proxy_static_method: {}", func_name);

        PROXY_REGISTRY.with(|registry_rc| {
            let registry = &*registry_rc.borrow();
            let proxy = registry.get(proxy_name).unwrap();
            if let Some(method) = proxy.static_methods.get(func_name) {
                let m_res: Result<JSValueRef, EsError> = method(args_vec);
                match m_res {
                    Ok(m_res_ref) => m_res_ref.consume_value_no_decr_rc(),
                    Err(e) => {
                        let err = format!("proxy_static_method failed: {}", e);
                        q_js_rt.report_ex(err.as_str())
                    }
                }
            } else {
                // return null if nothing was returned
                quickjs_utils::new_null()
            }
        })
    })
}

unsafe extern "C" fn proxy_static_set_prop(
    _ctx: *mut q::JSContext,
    _obj: q::JSValue,
    _atom: q::JSAtom,
    _value: q::JSValue,
    _receiver: q::JSValue,
    _flags: ::std::os::raw::c_int,
) -> ::std::os::raw::c_int {
    trace!("proxy_static_set_prop");
    0
}

unsafe extern "C" fn proxy_instance_set_prop(
    _ctx: *mut q::JSContext,
    _obj: q::JSValue,
    _atom: q::JSAtom,
    _value: q::JSValue,
    _receiver: q::JSValue,
    _flags: ::std::os::raw::c_int,
) -> ::std::os::raw::c_int {
    trace!("proxy_instance_set_prop");
    0
}
