use crate::quickjs_utils;
use crate::quickjs_utils::functions::new_native_function_q;
use crate::quickjs_utils::primitives::from_string;
use crate::quickjs_utils::{atoms, errors, functions, objects, parse_args, primitives};
use crate::quickjsrealmadapter::QuickJsRealmAdapter;
use crate::quickjsruntimeadapter::QuickJsRuntimeAdapter;
use crate::valueref::JSValueRef;
use hirofa_utils::js_utils::JsError;
use libquickjs_sys as q;
use log::trace;
use rand::{thread_rng, Rng};
use std::cell::RefCell;
use std::collections::HashMap;
use std::os::raw::{c_char, c_void};
use std::rc::Rc;

pub mod eventtarget;

pub type ProxyConstructor =
    dyn Fn(&QuickJsRealmAdapter, usize, Vec<JSValueRef>) -> Result<(), JsError> + 'static;
pub type ProxyFinalizer = dyn Fn(&QuickJsRealmAdapter, usize) + 'static;
pub type ProxyMethod =
    dyn Fn(&QuickJsRealmAdapter, &usize, Vec<JSValueRef>) -> Result<JSValueRef, JsError> + 'static;
pub type ProxyNativeMethod = q::JSCFunction;
pub type ProxyStaticMethod =
    dyn Fn(&QuickJsRealmAdapter, Vec<JSValueRef>) -> Result<JSValueRef, JsError> + 'static;
pub type ProxyStaticNativeMethod = q::JSCFunction;
pub type ProxyStaticGetter = dyn Fn(&QuickJsRealmAdapter) -> Result<JSValueRef, JsError> + 'static;
pub type ProxyStaticSetter =
    dyn Fn(&QuickJsRealmAdapter, JSValueRef) -> Result<(), JsError> + 'static;
pub type ProxyGetter =
    dyn Fn(&QuickJsRealmAdapter, &usize) -> Result<JSValueRef, JsError> + 'static;
pub type ProxySetter =
    dyn Fn(&QuickJsRealmAdapter, &usize, JSValueRef) -> Result<(), JsError> + 'static;

static CNAME: &str = "ProxyInstanceClass\0";
static SCNAME: &str = "ProxyStaticClass\0";

thread_local! {

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
    pub static PROXY_STATIC_CLASS_ID: RefCell<u32> = {
        let mut c_id: u32 = 0;
        let class_id: u32 = unsafe { q::JS_NewClassID(&mut c_id) };
        log::trace!("got static class id {}", class_id);

        PROXY_STATIC_CLASS_DEF.with(|cd_rc| {
            let class_def = &*cd_rc.borrow();
            QuickJsRuntimeAdapter::do_with(|q_js_rt| {
                let res = unsafe { q::JS_NewClass(q_js_rt.runtime, class_id, class_def) };
                log::trace!("new static class res {}", res);
                // todo res should be 0 for ok
            });
        });

        RefCell::new(class_id)
    };
    pub static PROXY_INSTANCE_CLASS_ID: RefCell<u32> = {
        let mut c_id: u32 = 0;
        let class_id: u32 = unsafe { q::JS_NewClassID(&mut c_id) };
        log::trace!("got class id {}", class_id);

        PROXY_INSTANCE_CLASS_DEF.with(|cd_rc| {
            let class_def = &*cd_rc.borrow();
            QuickJsRuntimeAdapter::do_with(|q_js_rt| {
                let res = unsafe { q::JS_NewClass(q_js_rt.runtime, class_id, class_def) };
                log::trace!("new class res {}", res);
                // todo res should be 0 for ok
            });
        });

        RefCell::new(class_id)
    };
}

const MAX_INSTANCE_NUM: usize = u32::MAX as usize;

fn next_id(q_ctx: &QuickJsRealmAdapter) -> usize {
    let mappings = &*q_ctx.proxy_instance_id_mappings.borrow();
    if mappings.len() == MAX_INSTANCE_NUM {
        panic!("too many instances"); // todo report ex
    }
    let mut rng = thread_rng();
    let mut r: usize = rng.gen();
    while mappings.contains_key(&r) {
        r += 1;
    }
    r
}

/// The Proxy struct can be used to create a class in JavaScript who's methods can be implemented in rust
/// # Example
/// ```rust
/// use quickjs_runtime::builder::QuickJsRuntimeBuilder;
/// use quickjs_runtime::reflection::Proxy;
/// use quickjs_runtime::quickjsrealmadapter::QuickJsRealmAdapter;
/// use quickjs_runtime::valueref::JSValueRef;
/// use std::cell::RefCell;
/// use std::collections::HashMap;
/// use quickjs_runtime::quickjs_utils::primitives;
/// use hirofa_utils::js_utils::Script;
/// use quickjs_runtime::esvalue::EsValueFacade;
///
/// struct MyFunkyStruct{
///     name: String
/// }
///
/// impl Drop for MyFunkyStruct {fn drop(&mut self) {
///         println!("Funky drop: {}", self.name.as_str());
///     }
/// }
///
/// thread_local! {
///    static INSTANCES: RefCell<HashMap<usize, MyFunkyStruct>> = RefCell::new(HashMap::new());
/// }
///
/// //create a new EsRuntime
/// let rt = QuickJsRuntimeBuilder::new().build();
///
/// // install our proxy class as com.hirofa.FunkyClass
/// rt.exe_rt_task_in_event_loop(|q_js_rt| {
///    let q_ctx = q_js_rt.get_main_context();
///    Proxy::new()
///    .namespace(vec!["com", "hirofa"])
///    .name("FunkyClass")
///    // the constructor is called when a script does new com.hirofa.FunkyClass, the reflection utils
///    // generate an instance_id which may be used to identify the instance
///    .constructor(|q_ctx: &QuickJsRealmAdapter, instance_id: usize, args: Vec<JSValueRef>| {
///        // we'll assume our script always constructs the Proxy with a single name argument
///        let name = primitives::to_string_q(q_ctx, &args[0]).ok().expect("bad constructor! bad!");
///        // create a new instance of our struct and store it in a map
///        let instance = MyFunkyStruct{name};
///        // store our struct in a thread_local map
///        INSTANCES.with(move |rc| {
///            let map = &mut *rc.borrow_mut();
///            map.insert(instance_id, instance);
///        });
///        // return Ok, or Err if the constructor failed (e.g. wrong args were passed)
///        Ok(())
///     })
///    // next we create a simple getName method, this will return a String
///    .method("getName", |q_ctx, instance_id, args| {
///        INSTANCES.with(move |rc| {
///            let map = & *rc.borrow();
///            let instance = map.get(instance_id).unwrap();
///            primitives::from_string_q(q_ctx, instance.name.as_str())
///        })
///    })
///    // and lastly (but very important) implement a finalizer so our rust struct may be dropped
///    .finalizer(|q_ctx, instance_id| {
///        INSTANCES.with(move |rc| {
///            let map = &mut *rc.borrow_mut();
///            map.remove(&instance_id);
///        });
///     })
///     // install the Proxy in the context
///    .install(q_ctx, true);      
/// });
///
/// match rt.eval_sync(Script::new("test_proxy.es",
///     "{let inst = new com.hirofa.FunkyClass('FooBar'); let name = inst.getName(); inst = null; name;}"
/// )) {
///     Ok(name_esvf) => {
///         // assert correct getName result
///         assert_eq!(name_esvf.get_str(), "FooBar");
///         let i_ct = INSTANCES.with(|rc| rc.borrow().len());
///         // assert instance was finalized
///         assert_eq!(i_ct, 0);
///     }
///     Err(e) => {
///         panic!("script failed: {}", e);
///     }
/// }
/// rt.gc_sync();
///
/// ```
pub struct Proxy {
    name: Option<String>,
    namespace: Option<Vec<String>>,
    constructor: Option<Box<ProxyConstructor>>,
    finalizers: Vec<Box<ProxyFinalizer>>,
    methods: HashMap<String, Box<ProxyMethod>>,
    native_methods: HashMap<String, ProxyNativeMethod>,
    static_methods: HashMap<String, Box<ProxyStaticMethod>>,
    static_native_methods: HashMap<String, ProxyStaticNativeMethod>,
    static_getters_setters: HashMap<String, (Box<ProxyStaticGetter>, Box<ProxyStaticSetter>)>,
    getters_setters: HashMap<String, (Box<ProxyGetter>, Box<ProxySetter>)>,
    is_event_target: bool,
    is_static_event_target: bool,
}

impl Default for crate::reflection::Proxy {
    fn default() -> Self {
        Self::new()
    }
}

/// get a proxy by class_name (namespace.ClassName)
pub fn get_proxy(q_ctx: &QuickJsRealmAdapter, class_name: &str) -> Option<Rc<Proxy>> {
    let registry = &*q_ctx.proxy_registry.borrow();
    registry.get(class_name).cloned()
}

impl Proxy {
    #[allow(dead_code)]
    pub fn new() -> Self {
        Proxy {
            name: None,
            namespace: None,
            constructor: None,
            finalizers: Default::default(),
            methods: Default::default(),
            native_methods: Default::default(),
            static_methods: Default::default(),
            static_native_methods: Default::default(),
            static_getters_setters: Default::default(),
            getters_setters: Default::default(),
            is_event_target: false,
            is_static_event_target: false,
        }
    }

    /// set the name of the proxy class
    /// this will indicate how to construct the class from script
    pub fn name(mut self, name: &str) -> Self {
        self.name = Some(name.to_string());
        self
    }
    /// set the namespace of the proxy class
    /// # Example
    /// ```
    /// use quickjs_runtime::reflection::Proxy;
    /// Proxy::new().namespace(vec!["com", "hirofa"]).name("SomeClass");
    /// ```
    /// means from script you can access the class by
    /// ```javascript
    /// let instance = new com.hirofa.SomeClass();
    /// ```
    pub fn namespace(mut self, namespace: Vec<&str>) -> Self {
        if namespace.is_empty() {
            self.namespace = None;
        } else {
            self.namespace = Some(namespace.iter().map(|s| s.to_string()).collect());
        }
        self
    }
    /// get the canonical classname of a Proxy
    /// # example
    /// ```
    /// use quickjs_runtime::reflection::Proxy;
    /// Proxy::new().namespace(vec!["com", "hirofa"]).name("SomeClass");
    /// ```
    /// will result in a class_name of "com.hirofa.SomeClass"
    pub fn get_class_name(&self) -> String {
        let cn = if let Some(n) = self.name.as_ref() {
            n.as_str()
        } else {
            "__nameless_class__"
        };
        if self.namespace.is_some() {
            format!("{}.{}", self.namespace.as_ref().unwrap().join("."), cn)
        } else {
            cn.to_string()
        }
    }
    /// add a constructor for the Proxy class
    /// this will enable a script to create a new instance of a Proxy class
    /// if omitted the Proxy class will not be constructable from script
    pub fn constructor<C>(mut self, constructor: C) -> Self
    where
        C: Fn(&QuickJsRealmAdapter, usize, Vec<JSValueRef>) -> Result<(), JsError> + 'static,
    {
        self.constructor = Some(Box::new(constructor));
        self
    }
    /// add a finalizer for the Proxy class
    /// this will be called when an instance of the Proxy class is dropped or garbage collected
    pub fn finalizer<C>(mut self, finalizer: C) -> Self
    where
        C: Fn(&QuickJsRealmAdapter, usize) + 'static,
    {
        self.finalizers.push(Box::new(finalizer));
        self
    }
    /// add a method to the Proxy class, this method will be available as a member of instances of the Proxy class
    pub fn method<M>(mut self, name: &str, method: M) -> Self
    where
        M: Fn(&QuickJsRealmAdapter, &usize, Vec<JSValueRef>) -> Result<JSValueRef, JsError>
            + 'static,
    {
        self.methods.insert(name.to_string(), Box::new(method));
        self
    }
    /// add a method to the Proxy class, this method will be available as a member of instances of the Proxy class
    pub fn native_method(mut self, name: &str, method: ProxyNativeMethod) -> Self {
        self.native_methods.insert(name.to_string(), method);
        self
    }
    /// add a static method to the Proxy class, this method will be available as a member of the Proxy class itself
    pub fn static_method<M>(mut self, name: &str, method: M) -> Self
    where
        M: Fn(&QuickJsRealmAdapter, Vec<JSValueRef>) -> Result<JSValueRef, JsError> + 'static,
    {
        self.static_methods
            .insert(name.to_string(), Box::new(method));
        self
    }
    /// add a static method to the Proxy class, this method will be available as a member of the Proxy class itself
    pub fn static_native_method(mut self, name: &str, method: ProxyStaticNativeMethod) -> Self {
        self.static_native_methods.insert(name.to_string(), method);
        self
    }

    /// add a static getter and setter to the Proxy class
    pub fn static_getter_setter<G, S>(mut self, name: &str, getter: G, setter: S) -> Self
    where
        G: Fn(&QuickJsRealmAdapter) -> Result<JSValueRef, JsError> + 'static,
        S: Fn(&QuickJsRealmAdapter, JSValueRef) -> Result<(), JsError> + 'static,
    {
        self.static_getters_setters
            .insert(name.to_string(), (Box::new(getter), Box::new(setter)));
        self
    }
    /// add a getter and setter to the Proxy class, these will be available as a member of an instance of this Proxy class
    pub fn getter_setter<G, S>(mut self, name: &str, getter: G, setter: S) -> Self
    where
        G: Fn(&QuickJsRealmAdapter, &usize) -> Result<JSValueRef, JsError> + 'static,
        S: Fn(&QuickJsRealmAdapter, &usize, JSValueRef) -> Result<(), JsError> + 'static,
    {
        self.getters_setters
            .insert(name.to_string(), (Box::new(getter), Box::new(setter)));
        self
    }
    /// indicate the Proxy class should implement the EventTarget interface, this will result in the addEventListener, removeEventListener and dispatchEvent methods to be available on instances of the Proxy class
    pub fn event_target(mut self) -> Self {
        self.is_event_target = true;
        self
    }
    /// indicate the Proxy class should implement the EventTarget interface, this will result in the addEventListener, removeEventListener and dispatchEvent methods to be available
    pub fn static_event_target(mut self) {
        self.is_static_event_target = true
    }
    /// install the Proxy class in a QuickJsContext, this is always needed as a final step to actually make the Proxy class work
    pub fn install(
        mut self,
        q_ctx: &QuickJsRealmAdapter,
        add_variable_to_global: bool,
    ) -> Result<JSValueRef, JsError> {
        if self.name.is_none() {
            return Err(JsError::new_str("Proxy needs a name"));
        }

        let prim_cn = self.get_class_name();
        self = self.method("Symbol.toPrimitive", move |q_ctx, id, _args| {
            let prim = primitives::from_string_q(
                q_ctx,
                format!("Proxy::instance({})::{}", id, prim_cn).as_str(),
            )?;
            Ok(prim)
        });
        let prim_cn = self.get_class_name();
        self = self.static_method("Symbol.toPrimitive", move |q_ctx, _args| {
            let prim = primitives::from_string_q(q_ctx, format!("Proxy::{}", prim_cn).as_str())?;
            Ok(prim)
        });

        let class_ref = self.install_class_prop(q_ctx, add_variable_to_global)?;
        eventtarget::impl_event_target(self).install_move_to_registry(q_ctx);

        Ok(class_ref)
    }

    fn install_move_to_registry(self, q_ctx: &QuickJsRealmAdapter) {
        let proxy = self;

        let reg_map = &mut *q_ctx.proxy_registry.borrow_mut();
        reg_map.insert(proxy.get_class_name(), Rc::new(proxy));
    }
    fn install_class_prop(
        &self,
        q_ctx: &QuickJsRealmAdapter,
        add_variable_to_global: bool,
    ) -> Result<JSValueRef, JsError> {
        // this creates a constructor function, adds it to the global scope and then makes an instance of the static_proxy_class its prototype so we can add static_getters_setters and static_methods

        log::trace!("reflection::Proxy::install_class_prop / 1");

        let static_class_id = PROXY_STATIC_CLASS_ID.with(|rc| *rc.borrow());

        log::trace!("reflection::Proxy::install_class_prop / 2");

        let constructor_ref = new_native_function_q(
            q_ctx,
            self.name.as_ref().unwrap().as_str(),
            Some(constructor),
            1,
            true,
        )?;

        log::trace!("reflection::Proxy::install_class_prop / 3");

        let class_val: q::JSValue =
            unsafe { q::JS_NewObjectClass(q_ctx.context, static_class_id as i32) };

        log::trace!("reflection::Proxy::install_class_prop / 4");

        let class_val_ref = JSValueRef::new(
            q_ctx.context,
            class_val,
            false,
            true,
            "reflection::Proxy::install_class_prop class_val",
        );

        assert_eq!(1, class_val_ref.get_ref_count());

        log::trace!("reflection::Proxy::install_class_prop / 5");

        if class_val_ref.is_exception() {
            return if let Some(e) = unsafe { QuickJsRealmAdapter::get_exception(q_ctx.context) } {
                Err(e)
            } else {
                Err(JsError::new_string(format!(
                    "could not create class:{}",
                    self.get_class_name()
                )))
            };
        }

        log::trace!("reflection::Proxy::install_class_prop / 6");

        unsafe {
            let res = q::JS_SetPrototype(
                q_ctx.context,
                *constructor_ref.borrow_value(),
                *class_val_ref.borrow_value(),
            );
            if res < 0 {
                return if let Some(err) = QuickJsRealmAdapter::get_exception(q_ctx.context) {
                    Err(err)
                } else {
                    Err(JsError::new_str("could not set class proto"))
                };
            }
        }

        assert_eq!(2, class_val_ref.get_ref_count());

        log::trace!("reflection::Proxy::install_class_prop / 7");

        objects::set_property2_q(
            q_ctx,
            &constructor_ref,
            "name",
            &primitives::from_string_q(q_ctx, &self.get_class_name())?,
            0,
        )?;

        // todo impl namespace here
        if add_variable_to_global {
            log::trace!("reflection::Proxy::install_class_prop / 8");
            let ns = if let Some(namespace) = &self.namespace {
                objects::get_namespace_q(
                    q_ctx,
                    namespace.iter().map(|s| s.as_str()).collect(),
                    true,
                )?
            } else {
                quickjs_utils::get_global_q(q_ctx)
            };

            log::trace!("reflection::Proxy::install_class_prop / 9");

            objects::set_property2_q(
                q_ctx,
                &ns,
                self.name.as_ref().unwrap().as_str(),
                &constructor_ref,
                0,
            )?;
        }
        log::trace!("reflection::Proxy::install_class_prop / 10");

        log::trace!("install_class_prop done");

        Ok(constructor_ref)
    }
}

pub fn new_instance2(
    proxy: &Proxy,
    q_ctx: &QuickJsRealmAdapter,
) -> Result<(usize, JSValueRef), JsError> {
    let instance_id = next_id(q_ctx);
    Ok((instance_id, new_instance3(proxy, instance_id, q_ctx)?))
}

pub(crate) fn new_instance3(
    proxy: &Proxy,
    instance_id: usize,
    q_ctx: &QuickJsRealmAdapter,
) -> Result<JSValueRef, JsError> {
    let ctx = q_ctx.context;
    let class_id = PROXY_INSTANCE_CLASS_ID.with(|rc| *rc.borrow());

    let class_val: q::JSValue = unsafe { q::JS_NewObjectClass(ctx, class_id as i32) };

    let class_name = proxy.get_class_name();

    let class_val_ref = JSValueRef::new(
        q_ctx.context,
        class_val,
        false,
        true,
        format!("reflection::Proxy; cn={}", class_name).as_str(),
    );

    if class_val_ref.is_exception() {
        return if let Some(e) = q_ctx.get_exception_ctx() {
            Err(JsError::new_string(format!(
                "could not create class:{} due to: {}",
                class_name, e
            )))
        } else {
            Err(JsError::new_string(format!(
                "could not create class:{}",
                class_name
            )))
        };
    }

    let mappings = &mut *q_ctx.proxy_instance_id_mappings.borrow_mut();
    assert!(!mappings.contains_key(&instance_id));

    let mut bx = Box::new(ProxyInstanceInfo {
        id: instance_id,
        class_name: proxy.get_class_name(),
        context_id: q_ctx.id.clone(),
    });

    let ibp: &mut ProxyInstanceInfo = &mut *bx;
    let info_ptr = ibp as *mut _ as *mut c_void;

    mappings.insert(instance_id, bx);
    unsafe { q::JS_SetOpaque(*class_val_ref.borrow_value(), info_ptr) };

    Ok(class_val_ref)
}

pub fn new_instance(
    class_name: &str,
    q_ctx: &QuickJsRealmAdapter,
) -> Result<(usize, JSValueRef), JsError> {
    // todo

    let registry = &*q_ctx.proxy_registry.borrow();

    if let Some(proxy) = registry.get(class_name) {
        // construct

        new_instance2(proxy, q_ctx)
    } else {
        Err(JsError::new_str("no such proxy"))
    }
}

#[allow(dead_code)]
unsafe extern "C" fn constructor(
    context: *mut q::JSContext,
    this_val: q::JSValue,
    argc: ::std::os::raw::c_int,
    argv: *mut q::JSValue,
) -> q::JSValue {
    log::trace!("constructor called, this_tag={}", this_val.tag);

    // this is the function we created earlier (the constructor)
    // so classname = this.name;
    let this_ref = JSValueRef::new(
        context,
        this_val,
        false,
        false,
        "reflection::constructor this_val",
    );
    QuickJsRuntimeAdapter::do_with(|q_js_rt| {
        let name_ref = objects::get_property(context, &this_ref, "name")
            .ok()
            .expect("name get failed");
        let class_name = functions::call_to_string(context, &name_ref)
            .ok()
            .expect("name.toString failed");

        let q_ctx = q_js_rt.get_quickjs_context(context);

        let registry = &*q_ctx.proxy_registry.borrow();
        if let Some(proxy) = registry.get(&class_name) {
            if let Some(constructor) = &proxy.constructor {
                // construct

                let args_vec = parse_args(context, argc, argv);
                let instance_id = next_id(q_ctx);
                let constructor_res = constructor(q_ctx, instance_id, args_vec);

                match constructor_res {
                    Ok(()) => {
                        let instance_ref_res = new_instance3(proxy, instance_id, q_ctx);

                        match instance_ref_res {
                            Ok(instance_ref) => instance_ref.clone_value_incr_rc(),
                            Err(e) => q_ctx.report_ex(
                                format!(
                                    "could not create proxy instance for {} due to {}",
                                    class_name, e
                                )
                                .as_str(),
                            ),
                        }
                    }
                    Err(es_err) => q_ctx.report_ex(
                        format!("constructor for {} failed with {}", class_name, es_err).as_str(),
                    ),
                }
            } else {
                q_ctx.report_ex("not a constructor")
            }
        } else {
            q_ctx.report_ex("no such proxy")
        }
    })
}

pub(crate) struct ProxyInstanceInfo {
    id: usize,
    class_name: String, // todo use unsafe to make these &str?
    context_id: String, // todo use unsafe to make these &str?
}

fn get_proxy_instance_info(val: &q::JSValue) -> &ProxyInstanceInfo {
    let class_id = PROXY_INSTANCE_CLASS_ID.with(|rc| *rc.borrow());
    let info_ptr: *mut c_void = unsafe { q::JS_GetOpaque(*val, class_id) };
    let info: &mut ProxyInstanceInfo = unsafe { &mut *(info_ptr as *mut ProxyInstanceInfo) };
    info
}

#[allow(dead_code)]
unsafe extern "C" fn finalizer(_rt: *mut q::JSRuntime, val: q::JSValue) {
    //todo
    log::trace!("finalizer called");

    let info: &ProxyInstanceInfo = get_proxy_instance_info(&val);
    trace!("finalize {}", info.id);

    QuickJsRuntimeAdapter::do_with(|q_js_rt| {
        let q_ctx = q_js_rt.get_context(&info.context_id);
        log::trace!("finalizer called, got q_ctx");
        let registry = &*q_ctx.proxy_registry.borrow();
        let proxy = registry.get(&info.class_name).unwrap();

        for finalizer in &proxy.finalizers {
            log::trace!("calling Proxy's finalizer");
            finalizer(q_ctx, info.id);
            log::trace!("after calling Proxy's finalizer");
        }

        {
            log::trace!("reflection::finalizer: remove from INSTANCE_ID_MAPPINGS");
            let id_map = &mut *q_ctx.proxy_instance_id_mappings.borrow_mut();
            let _ = id_map.remove(&info.id).expect("no such id to finalize");
            log::trace!("reflection::finalizer: remove from INSTANCE_ID_MAPPINGS -> done");
        }
        log::trace!("reflection::finalizer: 2");

        log::trace!("reflection::finalizer: 3, exit");
    });
}

#[allow(dead_code)]
unsafe extern "C" fn proxy_static_get_prop(
    context: *mut q::JSContext,
    obj: q::JSValue,
    atom: q::JSAtom,
    receiver: q::JSValue,
) -> q::JSValue {
    // static proxy class, not an instance
    trace!("proxy_static_get_prop");

    let _obj_ref = JSValueRef::new(
        context,
        obj,
        false,
        false,
        "reflection::proxy_static_get_prop obj",
    );
    let receiver_ref = JSValueRef::new(
        context,
        receiver,
        false,
        false,
        "reflection::proxy_static_get_prop receiver",
    );

    QuickJsRuntimeAdapter::do_with(|q_js_rt| {
        let q_ctx = q_js_rt.get_quickjs_context(context);

        let proxy_name_ref = objects::get_property(context, &receiver_ref, "name")
            .ok()
            .unwrap();
        let proxy_name = primitives::to_string(context, &proxy_name_ref)
            .ok()
            .unwrap();
        trace!("proxy_static_get_prop: {}", proxy_name);

        let prop_name = atoms::to_string2(context, &atom)
            .ok()
            .expect("could not get name");
        trace!("proxy_static_get_prop: prop: {}", prop_name);

        let registry = &*q_ctx.proxy_registry.borrow();
        if let Some(proxy) = registry.get(proxy_name.as_str()) {
            if proxy.static_methods.contains_key(&prop_name) {
                trace!("found method for {}", prop_name);

                let function_data_ref = from_string(context, prop_name.as_str())
                    .ok()
                    .expect("could not create function_data_ref");

                let func_ref = functions::new_native_function_data(
                    context,
                    Some(proxy_static_method),
                    prop_name.as_str(),
                    1,
                    function_data_ref,
                )
                .ok()
                .expect("could not create func");

                objects::set_property(context, &receiver_ref, prop_name.as_str(), &func_ref)
                    .ok()
                    .expect("set_property 9656738 failed");

                func_ref.clone_value_incr_rc()
            } else if let Some(native_static_method) = proxy.static_native_methods.get(&prop_name) {
                trace!("found static native method for {}", prop_name);

                let func_ref = functions::new_native_function(
                    context,
                    &prop_name,
                    *native_static_method,
                    1,
                    false,
                )
                .ok()
                .expect("could not create func");

                objects::set_property(context, &receiver_ref, prop_name.as_str(), &func_ref)
                    .ok()
                    .expect("set_property 36099 failed");

                func_ref.clone_value_incr_rc()
            } else if let Some(getter_setter) = proxy.static_getters_setters.get(&prop_name) {
                // call the getter
                let getter = &getter_setter.0;
                let res: Result<JSValueRef, JsError> = getter(q_ctx);
                match res {
                    Ok(g_val) => g_val.clone_value_incr_rc(),
                    Err(e) => {
                        let es = format!("proxy_static_get_prop failed: {}", e);
                        q_ctx.report_ex(es.as_str())
                    }
                }
            } else {
                quickjs_utils::new_null()
            }
        } else {
            q_ctx.report_ex("proxy class not found")
        }
    })
}

#[allow(dead_code)]
unsafe extern "C" fn proxy_instance_get_prop(
    context: *mut q::JSContext,
    obj: q::JSValue,
    atom: q::JSAtom,
    receiver: q::JSValue,
) -> q::JSValue {
    trace!("proxy_instance_get_prop");

    let _obj_ref = JSValueRef::new(
        context,
        obj,
        false,
        false,
        "reflection::proxy_instance_get_prop obj",
    );
    let receiver_ref = JSValueRef::new(
        context,
        receiver,
        false,
        false,
        "reflection::proxy_instance_get_prop receiver",
    );

    QuickJsRuntimeAdapter::do_with(|q_js_rt| {
        let q_ctx = q_js_rt.get_quickjs_context(context);

        let prop_name = atoms::to_string2(context, &atom)
            .ok()
            .expect("could not get name");
        trace!("proxy_instance_get_prop: {}", prop_name);

        let info = get_proxy_instance_info(&obj);

        trace!("obj_ref.classname = {}", info.class_name);

        // see if we have a matching method

        let registry = &*q_ctx.proxy_registry.borrow();
        let proxy = registry.get(&info.class_name).unwrap();
        if proxy.methods.contains_key(&prop_name) {
            trace!("found method for {}", prop_name);

            let function_data_ref = from_string(context, prop_name.as_str())
                .ok()
                .expect("could not create function_data_ref");

            let func_ref = functions::new_native_function_data(
                context,
                Some(proxy_instance_method),
                prop_name.as_str(),
                1,
                function_data_ref,
            )
            .ok()
            .expect("could not create func");

            objects::set_property(context, &receiver_ref, prop_name.as_str(), &func_ref)
                .ok()
                .expect("set_property 96385 failed"); // todo report ex

            func_ref.clone_value_incr_rc()
        } else if let Some(native_method) = proxy.native_methods.get(&prop_name) {
            trace!("found native method for {}", prop_name);

            let func_ref =
                functions::new_native_function(context, &prop_name, *native_method, 1, false)
                    .ok()
                    .expect("could not create func"); // tyodo report ex

            objects::set_property(context, &receiver_ref, prop_name.as_str(), &func_ref)
                .ok()
                .expect("set_property 49671 failed"); // todo report ex

            func_ref.clone_value_incr_rc()
        } else if let Some(getter_setter) = proxy.getters_setters.get(&prop_name) {
            // call the getter
            let getter = &getter_setter.0;
            let res: Result<JSValueRef, JsError> = getter(q_ctx, &info.id);
            match res {
                Ok(g_val) => g_val.clone_value_incr_rc(),
                Err(e) => {
                    let err = format!("proxy_instance_get_prop failed: {}", e);
                    q_ctx.report_ex(err.as_str())
                }
            }
        } else {
            // return null if nothing was returned
            quickjs_utils::new_null()
        }
    })

    // get constructor name
    // get proxy
    // get method or getter or setter
    // return native func (cache those?)
}
#[allow(dead_code)]
unsafe extern "C" fn proxy_instance_has_prop(
    _context: *mut q::JSContext,
    _obj: q::JSValue,
    _atom: q::JSAtom,
) -> ::std::os::raw::c_int {
    trace!("proxy_instance_has_prop");
    0
}
#[allow(dead_code)]
unsafe extern "C" fn proxy_static_has_prop(
    _context: *mut q::JSContext,
    _obj: q::JSValue,
    _atom: q::JSAtom,
) -> ::std::os::raw::c_int {
    trace!("proxy_static_has_prop");
    0
}

unsafe extern "C" fn proxy_instance_method(
    context: *mut q::JSContext,
    this_val: q::JSValue,
    argc: ::std::os::raw::c_int,
    argv: *mut q::JSValue,
    _magic: ::std::os::raw::c_int,
    func_data: *mut q::JSValue,
) -> q::JSValue {
    trace!("proxy_instance_method");
    QuickJsRuntimeAdapter::do_with(|q_js_rt| {
        let q_ctx = q_js_rt.get_quickjs_context(context);

        let proxy_instance_info: &ProxyInstanceInfo = get_proxy_instance_info(&this_val);

        let args_vec = parse_args(context, argc, argv);

        let func_name_ref = JSValueRef::new(
            context,
            *func_data,
            false,
            false,
            "reflection::proxy_instance_method func_data",
        );
        let func_name = primitives::to_string(context, &func_name_ref)
            .ok()
            .expect("could not to_string func_name_ref");

        trace!("proxy_instance_method: {}", func_name);

        let registry = &*q_ctx.proxy_registry.borrow();
        let proxy = registry
            .get(proxy_instance_info.class_name.as_str())
            .unwrap();
        if let Some(method) = proxy.methods.get(func_name.as_str()) {
            // todo report ex
            let m_res: Result<JSValueRef, JsError> =
                method(q_ctx, &proxy_instance_info.id, args_vec);

            match m_res {
                Ok(m_res_ref) => m_res_ref.clone_value_incr_rc(),
                Err(e) => {
                    let msg = format!("proxy_instance_method failed: {}", e.get_message());
                    let err = errors::new_error(context, e.get_name(), msg.as_str(), e.get_stack())
                        .ok()
                        .expect("create error failed");
                    errors::throw(context, err)
                }
            }
        } else {
            // return null if nothing was returned
            quickjs_utils::new_null()
        }
    })
}

#[allow(dead_code)]
unsafe extern "C" fn proxy_static_method(
    context: *mut q::JSContext,
    this_val: q::JSValue,
    argc: ::std::os::raw::c_int,
    argv: *mut q::JSValue,
    _magic: ::std::os::raw::c_int,
    func_data: *mut q::JSValue,
) -> q::JSValue {
    trace!("proxy_static_method");
    QuickJsRuntimeAdapter::do_with(|q_js_rt| {
        let q_ctx = q_js_rt.get_quickjs_context(context);
        let this_ref = JSValueRef::new(
            context,
            this_val,
            false,
            false,
            "reflection::proxy_static_method this_val",
        );

        let proxy_name_ref = objects::get_property(context, &this_ref, "name")
            .ok()
            .unwrap();
        let proxy_name = primitives::to_string(context, &proxy_name_ref)
            .ok()
            .expect("could not to_string classname");

        let args_vec = parse_args(context, argc, argv);

        let func_name_ref = JSValueRef::new(
            context,
            *func_data,
            false,
            false,
            "reflection::proxy_static_method func_data",
        );
        let func_name = primitives::to_string(context, &func_name_ref)
            .ok()
            .expect("could not to_string func_name_ref");

        trace!("proxy_static_method: {}", func_name);

        let registry = &*q_ctx.proxy_registry.borrow();
        let proxy = registry.get(proxy_name.as_str()).unwrap();
        if let Some(method) = proxy.static_methods.get(func_name.as_str()) {
            let m_res: Result<JSValueRef, JsError> = method(q_ctx, args_vec);
            match m_res {
                Ok(m_res_ref) => m_res_ref.clone_value_incr_rc(),
                Err(e) => {
                    let err = format!("proxy_static_method failed: {}", e);
                    q_ctx.report_ex(err.as_str())
                }
            }
        } else {
            // return null if nothing was returned
            quickjs_utils::new_null()
        }
    })
}

unsafe extern "C" fn proxy_static_set_prop(
    _context: *mut q::JSContext,
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
    _context: *mut q::JSContext,
    _obj: q::JSValue,
    _atom: q::JSAtom,
    _value: q::JSValue,
    _receiver: q::JSValue,
    _flags: ::std::os::raw::c_int,
) -> ::std::os::raw::c_int {
    trace!("proxy_instance_set_prop");
    0
}

#[cfg(test)]
pub mod tests {
    use crate::facades::tests::init_test_rt;
    use crate::quickjs_utils::{functions, primitives};
    use crate::reflection::Proxy;
    use hirofa_utils::js_utils::JsError;
    use hirofa_utils::js_utils::Script;
    use log::trace;
    use std::cell::RefCell;
    use std::collections::HashMap;
    use std::time::Duration;

    thread_local! {
        static TEST_INSTANCES: RefCell<HashMap<usize, String>> = RefCell::new(HashMap::new())
    }

    #[test]
    pub fn test_proxy1() {
        log::info!("> test_proxy");

        let rt = init_test_rt();
        rt.exe_rt_task_in_event_loop(|q_js_rt| {
            q_js_rt.gc();
            let q_ctx = q_js_rt.get_main_context();
            let _ = Proxy::new()
                .constructor(|_q_ctx, _id, _args| Ok(()))
                .name("Test")
                .install(q_ctx, true);
            q_ctx
                .eval(Script::new("test.es", "let t = new Test();"))
                .ok()
                .expect("script failed");
        });
    }

    #[test]
    pub fn test_to_string() {
        log::info!("> test_proxy");

        let rt = init_test_rt();
        rt.exe_rt_task_in_event_loop(|q_js_rt| {
            q_js_rt.gc();
            let q_ctx = q_js_rt.get_main_context();
            let _ = Proxy::new()
                .constructor(|_q_ctx, _id, _args| Ok(()))
                .namespace(vec!["com", "company"])
                .name("Test")
                .install(q_ctx, true);
            let res = q_ctx
                .eval(Script::new(
                    "test_tostring.es",
                    "com.company.Test + '-' + new com.company.Test()",
                ))
                .ok()
                .expect("script failed");
            let str = primitives::to_string_q(q_ctx, &res)
                .ok()
                .expect("could not tostring");
            assert!(str.starts_with("Proxy::com.company.Test-Proxy::instance("));
            assert!(str.ends_with(")::com.company.Test"));
        });
    }

    #[test]
    pub fn test_proxy() {
        log::info!("> test_proxy");

        let rt = init_test_rt();
        rt.exe_rt_task_in_event_loop(|q_js_rt| {
            let q_ctx = q_js_rt.get_main_context();
            let res = Proxy::new()
                .name("TestClass1")
                .constructor(|_context, id, _args| {
                    TEST_INSTANCES.with(|rc| {
                        let map = &mut *rc.borrow_mut();
                        map.insert(id, "hi".to_string())
                    });
                    Ok(())
                })
                .method("doIt", |_context, _obj_id, _args| {
                    Ok(primitives::from_i32(531))
                })
                .method("doIt2", |_context, _obj_id, _args| {
                    Err(JsError::new_str("aaargh"))
                })
                .getter_setter(
                    "gVar",
                    |_context, _id| Ok(primitives::from_i32(147)),
                    |_context, _id, _val| Ok(()),
                )
                .static_method("sDoIt", |_context, _args| Ok(primitives::from_i32(9876)))
                .static_method("sDoIt2", |_context, _args| Ok(primitives::from_i32(140)))
                .static_getter_setter(
                    "someThing",
                    |_context| {
                        trace!("static getter called, returning 754");
                        Ok(primitives::from_i32(754))
                    },
                    |q_ctx, val| {
                        trace!(
                            "static setter called, set to {}",
                            functions::call_to_string_q(q_ctx, &val)?
                        );
                        Ok(())
                    },
                )
                .finalizer(|_context, id| {
                    TEST_INSTANCES.with(|rc| {
                        let map = &mut *rc.borrow_mut();
                        let _ = map.remove(&id);
                    });
                    log::trace!("ran finalizer: {}", id);
                })
                .install(q_ctx, true);

            match res {
                Ok(_) => {}
                Err(e) => panic!("could not install proxy: {}", e),
            }
        });

        let i2_res = rt.eval_sync(Script::new(
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

        let i = rt.eval_sync(Script::new(
            "test_proxy2.es",
            "let tc1 = new TestClass1(1, true, 'abc'); let r = tc1.doIt(1, true, 'abc'); r = tc1.doIt(1, true, 'abc'); tc1 = null; r;"
        ))
            .ok()
            .expect("script failed");

        assert!(i.is_i32());
        assert_eq!(i.get_i32(), 531);

        let i3_res = rt.eval_sync(Script::new("test_proxy.es", "TestClass1.sDoIt();"));

        if i3_res.is_err() {
            panic!("script failed: {}", i3_res.err().unwrap());
        }
        let i3 = i3_res.ok().unwrap();

        assert!(i3.is_i32());
        assert_eq!(i3.get_i32(), 9876);

        let i4 = rt
            .eval_sync(Script::new(
                "test_proxy.es",
                "TestClass1.someThing = 1; TestClass1.someThing;",
            ))
            .ok()
            .expect("script failed");

        assert!(i4.is_i32());
        assert_eq!(i4.get_i32(), 754);

        let i5 = rt
            .eval_sync(Script::new(
                "test_proxy.es",
                "let tc5 = new TestClass1(); let r5 = tc5.gVar; tc5 = null; r5;",
            ))
            .ok()
            .expect("script failed");

        assert!(i5.is_i32());
        assert_eq!(i5.get_i32(), 147);

        let i6_res = rt.eval_sync(Script::new(
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
