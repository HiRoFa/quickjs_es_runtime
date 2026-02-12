//! utils for implementing proxy classes which can be used to use rust structs from JS (define method/getters/setters/etc)

use crate::jsutils::JsError;
use crate::quickjs_utils;
use crate::quickjs_utils::functions::new_native_function_q;
use crate::quickjs_utils::objects::{get_property, set_property2_q};
use crate::quickjs_utils::primitives::from_string;
use crate::quickjs_utils::{atoms, errors, functions, objects, parse_args, primitives};
use crate::quickjsrealmadapter::QuickJsRealmAdapter;
use crate::quickjsruntimeadapter::QuickJsRuntimeAdapter;
use crate::quickjsvalueadapter::QuickJsValueAdapter;
use libquickjs_sys as q;
use log::trace;
use rand::{thread_rng, Rng};
use std::cell::RefCell;
use std::collections::HashMap;
use std::os::raw::{c_char, c_void};
use std::rc::Rc;

pub type JsProxyInstanceId = usize;

pub mod eventtarget;

pub type ProxyConstructor = dyn Fn(
        &QuickJsRuntimeAdapter,
        &QuickJsRealmAdapter,
        usize,
        &[QuickJsValueAdapter],
    ) -> Result<(), JsError>
    + 'static;
pub type ProxyFinalizer = dyn Fn(&QuickJsRuntimeAdapter, &QuickJsRealmAdapter, usize) + 'static;
pub type ProxyMethod = dyn Fn(
        &QuickJsRuntimeAdapter,
        &QuickJsRealmAdapter,
        &usize,
        &[QuickJsValueAdapter],
    ) -> Result<QuickJsValueAdapter, JsError>
    + 'static;
pub type ProxyNativeMethod = q::JSCFunction;
pub type ProxyStaticMethod = dyn Fn(
        &QuickJsRuntimeAdapter,
        &QuickJsRealmAdapter,
        &[QuickJsValueAdapter],
    ) -> Result<QuickJsValueAdapter, JsError>
    + 'static;
pub type ProxyStaticNativeMethod = q::JSCFunction;
pub type ProxyStaticGetter = dyn Fn(&QuickJsRuntimeAdapter, &QuickJsRealmAdapter) -> Result<QuickJsValueAdapter, JsError>
    + 'static;
pub type ProxyStaticSetter = dyn Fn(&QuickJsRuntimeAdapter, &QuickJsRealmAdapter, QuickJsValueAdapter) -> Result<(), JsError>
    + 'static;
pub type ProxyStaticCatchAllGetter = dyn Fn(&QuickJsRuntimeAdapter, &QuickJsRealmAdapter, &str) -> Result<QuickJsValueAdapter, JsError>
    + 'static;
pub type ProxyStaticCatchAllSetter = dyn Fn(
        &QuickJsRuntimeAdapter,
        &QuickJsRealmAdapter,
        &str,
        QuickJsValueAdapter,
    ) -> Result<(), JsError>
    + 'static;
pub type ProxyGetter = dyn Fn(&QuickJsRuntimeAdapter, &QuickJsRealmAdapter, &usize) -> Result<QuickJsValueAdapter, JsError>
    + 'static;
pub type ProxyCatchAllGetter = dyn Fn(
        &QuickJsRuntimeAdapter,
        &QuickJsRealmAdapter,
        &usize,
        &str,
    ) -> Result<QuickJsValueAdapter, JsError>
    + 'static;
pub type ProxySetter = dyn Fn(
        &QuickJsRuntimeAdapter,
        &QuickJsRealmAdapter,
        &usize,
        QuickJsValueAdapter,
    ) -> Result<(), JsError>
    + 'static;
pub type ProxyCatchAllSetter = dyn Fn(
        &QuickJsRuntimeAdapter,
        &QuickJsRealmAdapter,
        &usize,
        &str,
        QuickJsValueAdapter,
    ) -> Result<(), JsError>
    + 'static;

static CNAME: &str = "ProxyInstanceClass\0";
static SCNAME: &str = "ProxyStaticClass\0";

thread_local! {

    #[cfg(feature = "quickjs-ng")]
    static PROXY_STATIC_EXOTIC: RefCell<q::JSClassExoticMethods> = RefCell::new(q::JSClassExoticMethods {
        get_own_property: None,
        get_own_property_names: None,
        delete_property: None,
        define_own_property: None,
        has_property: Some(proxy_static_has_prop),
        get_property: Some(proxy_static_get_prop),
        set_property: Some(proxy_static_set_prop),
    });
    #[cfg(feature = "bellard")]
    static PROXY_STATIC_EXOTIC: RefCell<q::JSClassExoticMethods> = RefCell::new(q::JSClassExoticMethods {
        get_own_property: None,
        get_own_property_names: None,
        delete_property: None,
        define_own_property: None,
        has_property: Some(proxy_static_has_prop),
        get_property: Some(proxy_static_get_prop),
        set_property: Some(proxy_static_set_prop),
        get_prototype: None,
        is_extensible: None,
        prevent_extensions: None,
        set_prototype: None
    });

    #[cfg(feature = "quickjs-ng")]
    static PROXY_INSTANCE_EXOTIC: RefCell<q::JSClassExoticMethods> = RefCell::new(q::JSClassExoticMethods {
        get_own_property: None,
        get_own_property_names: None,
        delete_property: None,
        define_own_property: None,
        has_property: Some(proxy_instance_has_prop),
        get_property: Some(proxy_instance_get_prop),
        set_property: Some(proxy_instance_set_prop),
    });

    #[cfg(feature = "bellard")]
    static PROXY_INSTANCE_EXOTIC: RefCell<q::JSClassExoticMethods> = RefCell::new(q::JSClassExoticMethods {
        get_own_property: None,
        get_own_property_names: None,
        delete_property: None,
        define_own_property: None,
        has_property: Some(proxy_instance_has_prop),
        get_property: Some(proxy_instance_get_prop),
        set_property: Some(proxy_instance_set_prop),
        get_prototype: None,
        is_extensible: None,
        prevent_extensions: None,
        set_prototype: None
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

        let class_id: u32 =
            QuickJsRuntimeAdapter::do_with(|q_js_rt| {
                q_js_rt.new_class_id()
            });

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

        let class_id: u32 =
            QuickJsRuntimeAdapter::do_with(|q_js_rt| {
                q_js_rt.new_class_id()
            });
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

pub(crate) fn init_statics() {
    PROXY_INSTANCE_CLASS_ID.with(|_rc| {
        //
    });
}

fn next_id(proxy: &Proxy) -> usize {
    let mappings = &*proxy.proxy_instance_id_mappings.borrow();
    if mappings.len() == MAX_INSTANCE_NUM {
        panic!("too many instances"); // todo report ex
    }
    let mut rng = thread_rng();
    let mut r: usize = rng.r#gen();
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
/// use quickjs_runtime::quickjsvalueadapter::QuickJsValueAdapter;
/// use std::cell::RefCell;
/// use std::collections::HashMap;
/// use quickjs_runtime::quickjs_utils::primitives;
/// use quickjs_runtime::jsutils::Script;
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
///    let q_ctx = q_js_rt.get_main_realm();
///    Proxy::new()
///    .namespace(&["com", "hirofa"])
///    .name("FunkyClass")
///    // the constructor is called when a script does new com.hirofa.FunkyClass, the reflection utils
///    // generate an instance_id which may be used to identify the instance
///    .constructor(|rt, q_ctx: &QuickJsRealmAdapter, instance_id: usize, args: &[QuickJsValueAdapter]| {
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
///    .method("getName", |rt, q_ctx, instance_id, args| {
///        INSTANCES.with(move |rc| {
///            let map = & *rc.borrow();
///            let instance = map.get(instance_id).unwrap();
///            primitives::from_string_q(q_ctx, instance.name.as_str())
///        })
///    })
///    // and lastly (but very important) implement a finalizer so our rust struct may be dropped
///    .finalizer(|rt, q_ctx, instance_id| {
///        INSTANCES.with(move |rc| {
///            let map = &mut *rc.borrow_mut();
///            map.remove(&instance_id);
///        });
///     })
///     // install the Proxy in the context
///    .install(q_ctx, true).expect("proxy install failed");      
/// });
///
/// match rt.eval_sync(None, Script::new("test_proxy.es",
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
    pub(crate) constructor: Option<Box<ProxyConstructor>>,
    finalizers: Vec<Box<ProxyFinalizer>>,
    methods: HashMap<String, Box<ProxyMethod>>,
    native_methods: HashMap<String, ProxyNativeMethod>,
    static_methods: HashMap<String, Box<ProxyStaticMethod>>,
    static_native_methods: HashMap<String, ProxyStaticNativeMethod>,
    static_getters_setters: HashMap<String, (Box<ProxyStaticGetter>, Box<ProxyStaticSetter>)>,
    getters_setters: HashMap<String, (Box<ProxyGetter>, Box<ProxySetter>)>,
    catch_all: Option<(Box<ProxyCatchAllGetter>, Box<ProxyCatchAllSetter>)>,
    static_catch_all: Option<(
        Box<ProxyStaticCatchAllGetter>,
        Box<ProxyStaticCatchAllSetter>,
    )>,
    is_event_target: bool,
    is_static_event_target: bool,
    pub(crate) proxy_instance_id_mappings: RefCell<HashMap<usize, Box<ProxyInstanceInfo>>>,
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
            catch_all: None,
            static_catch_all: None,
            is_event_target: false,
            is_static_event_target: false,
            proxy_instance_id_mappings: RefCell::new(Default::default()),
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
    /// Proxy::new().namespace(&["com", "hirofa"]).name("SomeClass");
    /// ```
    /// means from script you can access the class by
    /// ```javascript
    /// let instance = new com.hirofa.SomeClass();
    /// ```
    pub fn namespace(mut self, namespace: &[&str]) -> Self {
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
    /// Proxy::new().namespace(&["com", "hirofa"]).name("SomeClass");
    /// ```
    /// will result in a class_name of "com.hirofa.SomeClass"
    pub fn get_class_name(&self) -> String {
        let cn = if let Some(n) = self.name.as_ref() {
            n.as_str()
        } else {
            "__nameless_class__"
        };
        if let Some(ns) = &self.namespace {
            format!("{}.{}", ns.join("."), cn)
        } else {
            cn.to_string()
        }
    }
    /// add a constructor for the Proxy class
    /// this will enable a script to create a new instance of a Proxy class
    /// if omitted the Proxy class will not be constructable from script
    pub fn constructor<C>(mut self, constructor: C) -> Self
    where
        C: Fn(
                &QuickJsRuntimeAdapter,
                &QuickJsRealmAdapter,
                usize,
                &[QuickJsValueAdapter],
            ) -> Result<(), JsError>
            + 'static,
    {
        self.constructor = Some(Box::new(constructor));
        self
    }
    /// add a finalizer for the Proxy class
    /// this will be called when an instance of the Proxy class is dropped or garbage collected
    pub fn finalizer<C>(mut self, finalizer: C) -> Self
    where
        C: Fn(&QuickJsRuntimeAdapter, &QuickJsRealmAdapter, usize) + 'static,
    {
        self.finalizers.push(Box::new(finalizer));
        self
    }
    /// add a method to the Proxy class, this method will be available as a member of instances of the Proxy class
    pub fn method<M>(mut self, name: &str, method: M) -> Self
    where
        M: Fn(
                &QuickJsRuntimeAdapter,
                &QuickJsRealmAdapter,
                &usize,
                &[QuickJsValueAdapter],
            ) -> Result<QuickJsValueAdapter, JsError>
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
        M: Fn(
                &QuickJsRuntimeAdapter,
                &QuickJsRealmAdapter,
                &[QuickJsValueAdapter],
            ) -> Result<QuickJsValueAdapter, JsError>
            + 'static,
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
        G: Fn(&QuickJsRuntimeAdapter, &QuickJsRealmAdapter) -> Result<QuickJsValueAdapter, JsError>
            + 'static,
        S: Fn(
                &QuickJsRuntimeAdapter,
                &QuickJsRealmAdapter,
                QuickJsValueAdapter,
            ) -> Result<(), JsError>
            + 'static,
    {
        self.static_getters_setters
            .insert(name.to_string(), (Box::new(getter), Box::new(setter)));
        self
    }
    /// add a static getter and setter to the Proxy class
    pub fn static_catch_all_getter_setter<G, S>(mut self, getter: G, setter: S) -> Self
    where
        G: Fn(
                &QuickJsRuntimeAdapter,
                &QuickJsRealmAdapter,
                &str,
            ) -> Result<QuickJsValueAdapter, JsError>
            + 'static,
        S: Fn(
                &QuickJsRuntimeAdapter,
                &QuickJsRealmAdapter,
                &str,
                QuickJsValueAdapter,
            ) -> Result<(), JsError>
            + 'static,
    {
        self.static_catch_all = Some((Box::new(getter), Box::new(setter)));
        self
    }
    /// add a getter and setter to the Proxy class, these will be available as a member of an instance of this Proxy class
    pub fn getter_setter<G, S>(mut self, name: &str, getter: G, setter: S) -> Self
    where
        G: Fn(
                &QuickJsRuntimeAdapter,
                &QuickJsRealmAdapter,
                &usize,
            ) -> Result<QuickJsValueAdapter, JsError>
            + 'static,
        S: Fn(
                &QuickJsRuntimeAdapter,
                &QuickJsRealmAdapter,
                &usize,
                QuickJsValueAdapter,
            ) -> Result<(), JsError>
            + 'static,
    {
        self.getters_setters
            .insert(name.to_string(), (Box::new(getter), Box::new(setter)));
        self
    }
    /// add a getter and setter to the Proxy class, these will be available as a member of an instance of this Proxy class
    pub fn getter<G>(self, name: &str, getter: G) -> Self
    where
        G: Fn(
                &QuickJsRuntimeAdapter,
                &QuickJsRealmAdapter,
                &usize,
            ) -> Result<QuickJsValueAdapter, JsError>
            + 'static,
    {
        self.getter_setter(name, getter, |_rt, _realm, _id, _val| Ok(()))
    }
    /// add a catchall getter and setter to the Proxy class, these will be used for properties which are not specifically defined as getter, setter or method in this Proxy
    pub fn catch_all_getter_setter<G, S>(mut self, getter: G, setter: S) -> Self
    where
        G: Fn(
                &QuickJsRuntimeAdapter,
                &QuickJsRealmAdapter,
                &usize,
                &str,
            ) -> Result<QuickJsValueAdapter, JsError>
            + 'static,
        S: Fn(
                &QuickJsRuntimeAdapter,
                &QuickJsRealmAdapter,
                &usize,
                &str,
                QuickJsValueAdapter,
            ) -> Result<(), JsError>
            + 'static,
    {
        self.catch_all = Some((Box::new(getter), Box::new(setter)));

        self
    }
    /// indicate the Proxy class should implement the EventTarget interface, this will result in the addEventListener, removeEventListener and dispatchEvent methods to be available on instances of the Proxy class
    pub fn event_target(mut self) -> Self {
        self.is_event_target = true;
        self
    }
    /// indicate the Proxy class should implement the EventTarget interface, this will result in the addEventListener, removeEventListener and dispatchEvent methods to be available
    pub fn static_event_target(mut self) -> Self {
        self.is_static_event_target = true;
        self
    }
    /// install the Proxy class in a QuickJsContext, this is always needed as a final step to actually make the Proxy class work
    pub fn install(
        mut self,
        q_ctx: &QuickJsRealmAdapter,
        add_variable_to_global: bool,
    ) -> Result<QuickJsValueAdapter, JsError> {
        if self.name.is_none() {
            return Err(JsError::new_str("Proxy needs a name"));
        }

        let prim_cn = self.get_class_name();
        let prim_cn2 = prim_cn.clone();

        // todo turn these into native methods
        self = self.method("Symbol.toPrimitive", move |_rt, q_ctx, id, _args| {
            let prim = primitives::from_string_q(
                q_ctx,
                format!("Proxy::instance({id})::{prim_cn}").as_str(),
            )?;
            Ok(prim)
        });
        let prim_cn = self.get_class_name();
        self = self.static_method("Symbol.hasInstance", move |_rt, realm, args| {
            if args.len() == 1 {
                let instance = &args[0];
                if instance.is_proxy_instance() {
                    let info = realm.get_proxy_instance_info(instance)?;
                    if info.0.eq(prim_cn2.as_str()) {
                        return realm.create_boolean(true);
                    }
                }
            }
            realm.create_boolean(false)
        });
        self = self.static_method("Symbol.toPrimitive", move |_rt, q_ctx, _args| {
            let prim = primitives::from_string_q(q_ctx, format!("Proxy::{prim_cn}").as_str())?;
            Ok(prim)
        });

        let ret = self.install_class_prop(q_ctx, add_variable_to_global)?;
        eventtarget::impl_event_target(self).install_move_to_registry(q_ctx);

        Ok(ret)
    }

    fn install_move_to_registry(self, q_ctx: &QuickJsRealmAdapter) {
        let proxy = self;
        let reg_map = &mut *q_ctx.proxy_registry.borrow_mut();
        reg_map.insert(proxy.get_class_name(), Rc::new(proxy));
    }
    fn install_class_prop(
        &mut self,
        q_ctx: &QuickJsRealmAdapter,
        add_variable_to_global: bool,
    ) -> Result<QuickJsValueAdapter, JsError> {
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
            unsafe { q::JS_NewObjectClass(q_ctx.context, static_class_id as _) };

        log::trace!("reflection::Proxy::install_class_prop / 4");

        let class_val_ref = QuickJsValueAdapter::new(
            q_ctx.context,
            class_val,
            false,
            true,
            "reflection::Proxy::install_class_prop class_val",
        );

        #[cfg(feature = "bellard")]
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

        #[cfg(feature = "bellard")]
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
                let ns_str = namespace.iter().map(|s| s.as_str()).collect::<Vec<&str>>();
                objects::get_namespace_q(q_ctx, ns_str.as_slice(), true)?
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

        let proxy_constructor_refs = &mut *q_ctx.proxy_constructor_refs.borrow_mut();
        proxy_constructor_refs.insert(self.get_class_name(), constructor_ref.clone());

        log::trace!("install_class_prop done");

        Ok(constructor_ref)
    }
}

pub fn get_proxy_instance_proxy_and_instance_id_q(
    q_ctx: &QuickJsRealmAdapter,
    obj: &QuickJsValueAdapter,
) -> Option<(Rc<Proxy>, usize)> {
    if !is_proxy_instance_q(q_ctx, obj) {
        None
    } else {
        let info = get_proxy_instance_info(obj.borrow_value());
        let cn = info.class_name.as_str();
        let registry = &*q_ctx.proxy_registry.borrow();
        registry.get(cn).cloned().map(|proxy| (proxy, info.id))
    }
}

pub fn get_proxy_instance_id_q(
    q_ctx: &QuickJsRealmAdapter,
    obj: &QuickJsValueAdapter,
) -> Option<usize> {
    if !is_proxy_instance_q(q_ctx, obj) {
        None
    } else {
        let info = get_proxy_instance_info(obj.borrow_value());
        Some(info.id)
    }
}

/// Get the instance id of a proxy instance
/// # Safety
/// please make sure context is still valid
pub unsafe fn get_proxy_instance_id(
    ctx: *mut libquickjs_sys::JSContext,
    obj: &QuickJsValueAdapter,
) -> Option<usize> {
    if !is_proxy_instance(ctx, obj) {
        None
    } else {
        let info = get_proxy_instance_info(obj.borrow_value());
        Some(info.id)
    }
}

pub fn is_proxy_instance_q(q_ctx: &QuickJsRealmAdapter, obj: &QuickJsValueAdapter) -> bool {
    unsafe { is_proxy_instance(q_ctx.context, obj) }
}

/// check if an object is an instance of a Proxy class
/// # Safety
/// please make sure context is still valid
pub unsafe fn is_proxy_instance(ctx: *mut q::JSContext, obj: &QuickJsValueAdapter) -> bool {
    if !obj.is_object() {
        false
    } else {
        // workaround for instanceof not yet working
        let prop_res = get_property(ctx, obj, "__proxy__");
        if let Ok(prop) = prop_res {
            if prop.is_bool() && prop.to_bool() {
                return true;
            }
        }

        let class_id = PROXY_INSTANCE_CLASS_ID.with(|rc| *rc.borrow());
        let proxy_class_proto: q::JSValue = q::JS_GetClassProto(ctx, class_id);
        //let proto_ref: JSValueRef = JSValueRef::new(ctx, proxy_class_proto, false, false, "proxy_class_proto");

        let proxy_class_proto_obj = q::JS_GetPrototype(ctx, proxy_class_proto);
        let res = q::JS_IsInstanceOf(ctx, *obj.borrow_value(), proxy_class_proto_obj);

        if res == -1 {
            // log err
            if let Some(ex) = QuickJsRealmAdapter::get_exception(ctx) {
                log::error!("is_proxy_instance failed: {}", ex);
            } else {
                log::error!("is_proxy_instance failed");
            }
        }

        res > 0
    }
}

pub fn new_instance2(
    proxy: &Proxy,
    q_ctx: &QuickJsRealmAdapter,
) -> Result<(usize, QuickJsValueAdapter), JsError> {
    let instance_id = next_id(proxy);
    Ok((instance_id, new_instance3(proxy, instance_id, q_ctx)?))
}

pub(crate) fn new_instance3(
    proxy: &Proxy,
    instance_id: usize,
    q_ctx: &QuickJsRealmAdapter,
) -> Result<QuickJsValueAdapter, JsError> {
    let ctx = q_ctx.context;
    let class_id = PROXY_INSTANCE_CLASS_ID.with(|rc| *rc.borrow());

    let class_val: q::JSValue = unsafe { q::JS_NewObjectClass(ctx, class_id as _) };

    let class_name = proxy.get_class_name();

    trace!("creating new instance {} of {}", instance_id, class_name);

    let class_val_ref = QuickJsValueAdapter::new(
        q_ctx.context,
        class_val,
        false,
        true,
        format!("reflection::Proxy; cn={class_name}").as_str(),
    );

    if class_val_ref.is_exception() {
        return if let Some(e) = q_ctx.get_exception_ctx() {
            Err(JsError::new_string(format!(
                "could not create class:{class_name} due to: {e}"
            )))
        } else {
            Err(JsError::new_string(format!(
                "could not create class:{class_name}"
            )))
        };
    }

    let mappings = &mut *proxy.proxy_instance_id_mappings.borrow_mut();
    assert!(!mappings.contains_key(&instance_id));

    let mut bx = Box::new(ProxyInstanceInfo {
        id: instance_id,
        class_name: proxy.get_class_name(),
        context_id: q_ctx.id.clone(),
    });

    let ibp: &mut ProxyInstanceInfo = &mut bx;
    let info_ptr = ibp as *mut _ as *mut c_void;

    mappings.insert(instance_id, bx);
    unsafe { q::JS_SetOpaque(*class_val_ref.borrow_value(), info_ptr) };

    // todo this is a workaround.. i need to set a prototype for classes using JS_setClassProto per context on init..
    set_property2_q(
        q_ctx,
        &class_val_ref,
        "__proxy__",
        &primitives::from_bool(true),
        0,
    )?;

    let proxy_constructor_refs = &*q_ctx.proxy_constructor_refs.borrow();

    let constructor = proxy_constructor_refs
        .get(&class_name)
        .expect("proxy was not installed properly");

    set_property2_q(q_ctx, &class_val_ref, "constructor", constructor, 0)?;

    Ok(class_val_ref)
}

pub fn new_instance(
    class_name: &str,
    q_ctx: &QuickJsRealmAdapter,
) -> Result<(usize, QuickJsValueAdapter), JsError> {
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
    let this_ref = QuickJsValueAdapter::new(
        context,
        this_val,
        false,
        false,
        "reflection::constructor this_val",
    );
    QuickJsRuntimeAdapter::do_with(|q_js_rt| {
        let name_ref = objects::get_property(context, &this_ref, "name").expect("name get failed");
        let class_name =
            functions::call_to_string(context, &name_ref).expect("name.toString failed");

        let q_ctx = q_js_rt.get_quickjs_context(context);

        let registry = &*q_ctx.proxy_registry.borrow();
        if let Some(proxy) = registry.get(&class_name) {
            if let Some(constructor) = &proxy.constructor {
                // construct

                let args_vec = parse_args(context, argc, argv);
                let instance_id = next_id(proxy);
                let constructor_res = constructor(q_js_rt, q_ctx, instance_id, &args_vec);

                match constructor_res {
                    Ok(()) => {
                        let instance_ref_res = new_instance3(proxy, instance_id, q_ctx);

                        match instance_ref_res {
                            Ok(instance_ref) => instance_ref.clone_value_incr_rc(),

                            Err(e) => q_ctx.report_ex(
                                format!(
                                    "could not create proxy instance for {class_name} due to {e}"
                                )
                                .as_str(),
                            ),
                        }
                    }
                    Err(es_err) => q_ctx.report_ex(
                        format!("constructor for {class_name} failed with {es_err}").as_str(),
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
    class_name: String, // todo, store all proxies in an autoidmap with a usize as key and store proxy_class_id here instead of string
    context_id: String, // todo store all context ids in an autoidmap with a usize as key and store context_id here instead of string
}

fn get_proxy_instance_info(val: &q::JSValue) -> &ProxyInstanceInfo {
    let class_id = PROXY_INSTANCE_CLASS_ID.with(|rc| *rc.borrow());
    let info_ptr: *mut c_void = unsafe { q::JS_GetOpaque(*val, class_id) };
    let info: &mut ProxyInstanceInfo = unsafe { &mut *(info_ptr as *mut ProxyInstanceInfo) };
    info
}

#[allow(dead_code)]
unsafe extern "C" fn finalizer(_rt: *mut q::JSRuntime, val: q::JSValue) {
    log::trace!("finalizer called");

    let info: &ProxyInstanceInfo = get_proxy_instance_info(&val);
    trace!(
        "finalize id:{} class:{} context:{}",
        info.id,
        info.class_name,
        info.context_id
    );

    QuickJsRuntimeAdapter::do_with(|q_js_rt| {
        let q_ctx = q_js_rt.get_context(&info.context_id);
        log::trace!("finalizer called, got q_ctx");
        let registry = &*q_ctx.proxy_registry.borrow();
        let proxy = registry.get(&info.class_name).unwrap();

        for finalizer in &proxy.finalizers {
            log::trace!("calling Proxy's finalizer");
            finalizer(q_js_rt, q_ctx, info.id);
            log::trace!("after calling Proxy's finalizer");
        }

        {
            log::trace!("reflection::finalizer: remove from INSTANCE_ID_MAPPINGS");
            let id_map = &mut *proxy.proxy_instance_id_mappings.borrow_mut();
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

    let _obj_ref = QuickJsValueAdapter::new(
        context,
        obj,
        false,
        false,
        "reflection::proxy_static_get_prop obj",
    );
    let receiver_ref = QuickJsValueAdapter::new(
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

        let prop_name = atoms::to_string2(context, &atom).expect("could not get name");
        trace!("proxy_static_get_prop: prop: {}", prop_name);

        let registry = &*q_ctx.proxy_registry.borrow();
        if let Some(proxy) = registry.get(proxy_name.as_str()) {
            if proxy.static_methods.contains_key(prop_name.as_str()) {
                trace!("found method for {}", prop_name);

                let function_data_ref = from_string(context, prop_name.as_str())
                    .expect("could not create function_data_ref");

                let func_ref = functions::new_native_function_data(
                    context,
                    Some(proxy_static_method),
                    prop_name.as_str(),
                    1,
                    function_data_ref,
                )
                .expect("could not create func");

                objects::set_property(context, &receiver_ref, prop_name.as_str(), &func_ref)
                    .expect("set_property 9656738 failed");

                func_ref.clone_value_incr_rc()
            } else if let Some(native_static_method) =
                proxy.static_native_methods.get(prop_name.as_str())
            {
                trace!("found static native method for {}", prop_name);

                let func_ref = functions::new_native_function(
                    context,
                    prop_name.as_str(),
                    *native_static_method,
                    1,
                    false,
                )
                .expect("could not create func");

                objects::set_property(context, &receiver_ref, prop_name.as_str(), &func_ref)
                    .expect("set_property 36099 failed");

                func_ref.clone_value_incr_rc()
            } else if let Some(getter_setter) = proxy.static_getters_setters.get(prop_name.as_str())
            {
                // call the getter
                let getter = &getter_setter.0;
                let res: Result<QuickJsValueAdapter, JsError> = getter(q_js_rt, q_ctx);
                match res {
                    Ok(g_val) => g_val.clone_value_incr_rc(),
                    Err(e) => {
                        let es = format!("proxy_static_get_prop failed: {e}");
                        q_ctx.report_ex(es.as_str())
                    }
                }
            } else if let Some(catch_all_getter_setter) = &proxy.static_catch_all {
                // call the getter
                let getter = &catch_all_getter_setter.0;
                let res: Result<QuickJsValueAdapter, JsError> =
                    getter(q_js_rt, q_ctx, prop_name.as_str());
                match res {
                    Ok(g_val) => g_val.clone_value_incr_rc(),
                    Err(e) => {
                        let es = format!("proxy_static_get_prop failed: {e}");
                        q_ctx.report_ex(es.as_str())
                    }
                }
            } else {
                quickjs_utils::new_undefined()
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

    let _obj_ref = QuickJsValueAdapter::new(
        context,
        obj,
        false,
        false,
        "reflection::proxy_instance_get_prop obj",
    );
    let receiver_ref = QuickJsValueAdapter::new(
        context,
        receiver,
        false,
        false,
        "reflection::proxy_instance_get_prop receiver",
    );

    QuickJsRuntimeAdapter::do_with(|q_js_rt| {
        let q_ctx = q_js_rt.get_quickjs_context(context);

        let prop_name = atoms::to_string2(context, &atom).expect("could not get name");
        trace!("proxy_instance_get_prop: {}", prop_name);

        let info = get_proxy_instance_info(&obj);

        trace!("obj_ref.classname = {}", info.class_name);

        // see if we have a matching method

        let registry = &*q_ctx.proxy_registry.borrow();
        let proxy = registry.get(&info.class_name).unwrap();
        if proxy.methods.contains_key(prop_name.as_str()) {
            trace!("found method for {}", prop_name);

            let function_data_ref = from_string(context, prop_name.as_str())
                .expect("could not create function_data_ref");

            let func_ref = functions::new_native_function_data(
                context,
                Some(proxy_instance_method),
                prop_name.as_str(),
                1,
                function_data_ref,
            )
            .expect("could not create func");

            objects::set_property(context, &receiver_ref, prop_name.as_str(), &func_ref)
                .expect("set_property 96385 failed"); // todo report ex

            func_ref.clone_value_incr_rc()
        } else if let Some(native_method) = proxy.native_methods.get(prop_name.as_str()) {
            trace!("found native method for {}", prop_name);

            let func_ref = functions::new_native_function(
                context,
                prop_name.as_str(),
                *native_method,
                1,
                false,
            )
            .expect("could not create func"); // tyodo report ex

            objects::set_property(context, &receiver_ref, prop_name.as_str(), &func_ref)
                .expect("set_property 49671 failed"); // todo report ex

            func_ref.clone_value_incr_rc()
        } else if let Some(getter_setter) = proxy.getters_setters.get(prop_name.as_str()) {
            // call the getter
            let getter = &getter_setter.0;
            let res: Result<QuickJsValueAdapter, JsError> = getter(q_js_rt, q_ctx, &info.id);
            match res {
                Ok(g_val) => g_val.clone_value_incr_rc(),
                Err(e) => {
                    let msg = format!("proxy_instance_get failed: {}", e.get_message());
                    let nat_stack = format!(
                        "    at Proxy instance getter [{}]\n{}",
                        prop_name,
                        e.get_stack()
                    );
                    let err =
                        errors::new_error(context, e.get_name(), msg.as_str(), nat_stack.as_str())
                            .expect("create error failed");
                    errors::throw(context, err)
                }
            }
        } else if let Some(catch_all_getter_setter) = &proxy.catch_all {
            // call the getter
            let getter = &catch_all_getter_setter.0;
            let res: Result<QuickJsValueAdapter, JsError> =
                getter(q_js_rt, q_ctx, &info.id, prop_name.as_str());
            match res {
                Ok(g_val) => g_val.clone_value_incr_rc(),
                Err(e) => {
                    let msg = format!("proxy_instance_catch_all_get failed: {}", e.get_message());
                    let nat_stack = format!(
                        "    at Proxy instance getter [{}]\n{}",
                        prop_name,
                        e.get_stack()
                    );
                    let err =
                        errors::new_error(context, e.get_name(), msg.as_str(), nat_stack.as_str())
                            .expect("create error failed");
                    errors::throw(context, err)
                }
            }
        } else {
            // return null if nothing was returned
            quickjs_utils::new_undefined()
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
    todo!()
}
#[allow(dead_code)]
unsafe extern "C" fn proxy_static_has_prop(
    _context: *mut q::JSContext,
    _obj: q::JSValue,
    _atom: q::JSAtom,
) -> ::std::os::raw::c_int {
    todo!()
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

        let func_name_ref = QuickJsValueAdapter::new(
            context,
            *func_data,
            false,
            false,
            "reflection::proxy_instance_method func_data",
        );
        let func_name = primitives::to_string(context, &func_name_ref)
            .expect("could not to_string func_name_ref");

        trace!("proxy_instance_method: {}", func_name);

        let registry = &*q_ctx.proxy_registry.borrow();
        let proxy = registry
            .get(proxy_instance_info.class_name.as_str())
            .unwrap();
        if let Some(method) = proxy.methods.get(func_name.as_str()) {
            // todo report ex
            let m_res: Result<QuickJsValueAdapter, JsError> =
                method(q_js_rt, q_ctx, &proxy_instance_info.id, &args_vec);

            match m_res {
                Ok(m_res_ref) => m_res_ref.clone_value_incr_rc(),
                Err(e) => {
                    let msg = format!("proxy_instance_method failed: {}", e.get_message());
                    let nat_stack = format!(
                        "    at Proxy instance method [{}]\n{}",
                        func_name,
                        e.get_stack()
                    );
                    let err =
                        errors::new_error(context, e.get_name(), msg.as_str(), nat_stack.as_str())
                            .expect("create error failed");
                    errors::throw(context, err)
                }
            }
        } else {
            // return null if nothing was returned
            quickjs_utils::new_undefined()
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
        let this_ref = QuickJsValueAdapter::new(
            context,
            this_val,
            false,
            false,
            "reflection::proxy_static_method this_val",
        );

        let proxy_name_ref = objects::get_property(context, &this_ref, "name")
            .ok()
            .unwrap();
        let proxy_name =
            primitives::to_string(context, &proxy_name_ref).expect("could not to_string classname");

        let args_vec = parse_args(context, argc, argv);

        let func_name_ref = QuickJsValueAdapter::new(
            context,
            *func_data,
            false,
            false,
            "reflection::proxy_static_method func_data",
        );
        let func_name = primitives::to_string(context, &func_name_ref)
            .expect("could not to_string func_name_ref");

        trace!("proxy_static_method: {}", func_name);

        let registry = &*q_ctx.proxy_registry.borrow();
        let proxy = registry.get(proxy_name.as_str()).unwrap();
        if let Some(method) = proxy.static_methods.get(func_name.as_str()) {
            let m_res: Result<QuickJsValueAdapter, JsError> = method(q_js_rt, q_ctx, &args_vec);
            match m_res {
                Ok(m_res_ref) => m_res_ref.clone_value_incr_rc(),
                Err(e) => {
                    let msg = format!("proxy_static_method failed: {}", e.get_message());
                    let nat_stack = format!(
                        "    at Proxy static method [{}]\n{}",
                        func_name,
                        e.get_stack()
                    );
                    let err =
                        errors::new_error(context, e.get_name(), msg.as_str(), nat_stack.as_str())
                            .expect("create error failed");
                    errors::throw(context, err)
                }
            }
        } else {
            // return null if nothing was returned
            quickjs_utils::new_undefined()
        }
    })
}

unsafe extern "C" fn proxy_static_set_prop(
    context: *mut q::JSContext,
    _obj: q::JSValue,
    atom: q::JSAtom,
    value: q::JSValue,
    receiver: q::JSValue,
    _flags: ::std::os::raw::c_int,
) -> ::std::os::raw::c_int {
    trace!("proxy_static_set_prop");

    let value_ref = QuickJsValueAdapter::new(
        context,
        value,
        false,
        false,
        "reflection::proxy_static_set_prop value",
    );
    let receiver_ref = QuickJsValueAdapter::new(
        context,
        receiver,
        false,
        false,
        "reflection::proxy_static_set_prop value",
    );

    QuickJsRuntimeAdapter::do_with(|rt| {
        let realm = rt.get_quickjs_context(context);

        let prop_name = atoms::to_string2(context, &atom).expect("could not get name");
        trace!("proxy_static_set_prop: {}", prop_name);

        // see if we have a matching gettersetter

        let proxy_name_ref = objects::get_property(context, &receiver_ref, "name")
            .ok()
            .unwrap();
        let proxy_name = primitives::to_string(context, &proxy_name_ref)
            .ok()
            .unwrap();
        trace!("proxy_static_set_prop: {}", proxy_name);

        let registry = &*realm.proxy_registry.borrow();
        if let Some(proxy) = registry.get(proxy_name.as_str()) {
            if let Some(getter_setter) = proxy.static_getters_setters.get(prop_name.as_str()) {
                // call the setter
                let setter = &getter_setter.1;
                let res: Result<(), JsError> = setter(rt, realm, value_ref);
                match res {
                    Ok(_) => 0,
                    Err(e) => {
                        // fail, todo do i need ex?
                        let err = format!("proxy_static_set_prop failed: {e}");
                        log::error!("{}", err);
                        let _ = realm.report_ex(err.as_str());
                        -1
                    }
                }
            } else if let Some(catch_all_getter_setter) = &proxy.static_catch_all {
                // call the setter
                let setter = &catch_all_getter_setter.1;
                let res: Result<(), JsError> = setter(rt, realm, prop_name.as_str(), value_ref);
                match res {
                    Ok(_) => 0,
                    Err(e) => {
                        // fail, todo do i need ex?
                        let err = format!("proxy_static_set_prop failed: {e}");
                        log::error!("{}", err);
                        let _ = realm.report_ex(err.as_str());
                        -1
                    }
                }
            } else {
                let receiver_ref = QuickJsValueAdapter::new(
                    context,
                    receiver,
                    false,
                    false,
                    "reflection::proxy_static_set_prop receiver",
                );

                match realm.set_object_property(&receiver_ref, prop_name.as_str(), &value_ref) {
                    Ok(()) => 0,
                    Err(e) => {
                        let err = format!("proxy_static_set_prop failed, {}", e);
                        log::error!("{}", err);
                        let _ = realm.report_ex(err.as_str());
                        -1
                    }
                }
                /*
                let err = format!("proxy_static_set_prop failed, no handler found for proxy_static_set_prop: {}", prop_name);
                log::error!("{}", err);
                let _ = q_ctx.report_ex(err.as_str());
                -1

                 */
            }
        } else {
            let err = "proxy_static_set_prop failed, no proxy found";
            log::error!("{}", err);
            let _ = realm.report_ex(err);
            -1
        }
    })
}

unsafe extern "C" fn proxy_instance_set_prop(
    context: *mut q::JSContext,
    obj: q::JSValue,
    atom: q::JSAtom,
    value: q::JSValue,
    receiver: q::JSValue,
    _flags: ::std::os::raw::c_int,
) -> ::std::os::raw::c_int {
    trace!("proxy_instance_set_prop");

    let value_ref = QuickJsValueAdapter::new(
        context,
        value,
        false,
        false,
        "reflection::proxy_instance_set_prop value",
    );

    QuickJsRuntimeAdapter::do_with(|rt| {
        let realm = rt.get_quickjs_context(context);

        let prop_name = atoms::to_string2(context, &atom).expect("could not get name");
        trace!("proxy_instance_set_prop: {}", prop_name);

        let info = get_proxy_instance_info(&obj);

        trace!("obj_ref.classname = {}", info.class_name);

        // see if we have a matching gettersetter

        let registry = &*realm.proxy_registry.borrow();
        let proxy = registry.get(&info.class_name).unwrap();

        if let Some(getter_setter) = proxy.getters_setters.get(prop_name.as_str()) {
            // call the setter
            let setter = &getter_setter.1;
            let res: Result<(), JsError> = setter(rt, realm, &info.id, value_ref);
            match res {
                Ok(_) => 0,
                Err(e) => {
                    // fail, todo do i need ex?
                    let err = format!("proxy_instance_set_prop failed: {e}");
                    log::error!("{}", err);
                    let _ = realm.report_ex(err.as_str());
                    -1
                }
            }
        } else if let Some(catch_all_getter_setter) = &proxy.catch_all {
            // call the setter
            let setter = &catch_all_getter_setter.1;
            let res: Result<(), JsError> =
                setter(rt, realm, &info.id, prop_name.as_str(), value_ref);
            match res {
                Ok(_) => 0,
                Err(e) => {
                    // fail, todo do i need ex?
                    let err = format!("proxy_instance_set_prop failed: {e}");
                    log::error!("{}", err);
                    let _ = realm.report_ex(err.as_str());
                    -1
                }
            }
        } else {
            // if not handler just add to receiver

            let receiver_ref = QuickJsValueAdapter::new(
                context,
                receiver,
                false,
                false,
                "reflection::proxy_instance_set_prop receiver",
            );

            match realm.set_object_property(&receiver_ref, prop_name.as_str(), &value_ref) {
                Ok(()) => 0,
                Err(e) => {
                    let err = format!("proxy_instance_set_prop failed, {}", e);
                    log::error!("{}", err);
                    let _ = realm.report_ex(err.as_str());
                    -1
                }
            }
            /*
            let err = format!(
                "proxy_instance_set_prop failed, no handler found for proxy_instance_set_prop: {}",
                prop_name
            );
            log::error!("{}", err);
            let _ = realm.report_ex(err.as_str());
            -1

             */
        }
    })
}

#[cfg(test)]
pub mod tests {
    use crate::facades::tests::init_test_rt;
    use crate::jsutils::JsError;
    use crate::jsutils::Script;
    use crate::quickjs_utils::objects::create_object_q;
    use crate::quickjs_utils::{functions, primitives};
    use crate::reflection::{
        get_proxy_instance_proxy_and_instance_id_q, is_proxy_instance_q, Proxy,
        PROXY_INSTANCE_CLASS_ID,
    };
    use libquickjs_sys as q;
    use log::trace;
    use std::cell::RefCell;
    use std::collections::HashMap;
    use std::panic;
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
            let q_ctx = q_js_rt.get_main_realm();
            let _ = Proxy::new()
                .constructor(|_q_js_rt, _q_ctx, _id, _args| Ok(()))
                .name("Test")
                .install(q_ctx, true);
            q_ctx
                .eval(Script::new("test.es", "let t = new Test();"))
                .expect("script failed");
        });
    }

    #[test]
    pub fn test_proxy_ex() {
        log::info!("> test_proxy");

        let rt = init_test_rt();
        let err = rt.exe_rt_task_in_event_loop(|q_js_rt| {
            q_js_rt.gc();
            let q_ctx = q_js_rt.get_main_realm();
            let _ = Proxy::new()
                .constructor(|_q_js_rt, _q_ctx, _id, _args| Ok(()))
                .method("run", |_rt, _realm, _instance_id, _args| {
                    Err(JsError::new_str("cant run"))
                })
                .name("Test")
                .install(q_ctx, true);
            let err = q_ctx
                .eval(Script::new("test.es", "let t = new Test(); \nt.run();"))
                .expect_err("script failed");

            format!("{err}")
        });

        assert!(err.contains("test.es:2"));
        assert!(err.contains("at Proxy instance method [run]"));
        assert!(err.contains("cant run"));
    }

    #[test]
    pub fn test_proxy_instanceof() {
        log::info!("> test_proxy_instanceof");

        let rt = init_test_rt();
        rt.exe_rt_task_in_event_loop(|q_js_rt| {
            q_js_rt.gc();
            let q_ctx = q_js_rt.get_main_realm();
            let _ = Proxy::new()
                .constructor(|_rt, _q_ctx, _id, _args| Ok(()))
                .namespace(&["com", "company"])
                .name("Test")
                .install(q_ctx, true);
            let res = q_ctx
                .eval(Script::new("test_tostring.es", "new com.company.Test()"))
                .expect("script failed");
            assert!(is_proxy_instance_q(q_ctx, &res));
            let info = get_proxy_instance_proxy_and_instance_id_q(q_ctx, &res)
                .expect("could not get info");
            let id = info.1;
            let p = info.0;
            println!("id={id}");
            assert_eq!(p.get_class_name().as_str(), "com.company.Test");

            let some_obj = create_object_q(q_ctx).expect("could not create obj");
            assert!(some_obj.is_object());

            let class_id = PROXY_INSTANCE_CLASS_ID.with(|rc| *rc.borrow());
            let proxy_class_proto: q::JSValue =
                unsafe { q::JS_GetClassProto(q_ctx.context, class_id) };
            //println!("proxy_class_proto = {}", proxy_class_proto);
            let res = unsafe {
                q::JS_IsInstanceOf(q_ctx.context, *some_obj.borrow_value(), proxy_class_proto) != 0
            };
            println!("res = {res}");
            let res2 = is_proxy_instance_q(q_ctx, &some_obj);
            println!("res2 = {res2}");
            assert!(!res2);
        });
    }

    #[test]
    pub fn test_rest_props() {
        log::info!("> test_rest_props");

        let rt = init_test_rt();
        rt.exe_rt_task_in_event_loop(|q_js_rt| {
            q_js_rt.gc();
            let realm = q_js_rt.get_main_realm();
            let _ = Proxy::new()
                .constructor(|_rt, _q_ctx, _id, _args| Ok(()))
                .namespace(&["com", "company"])
                .name("Test")
                .install(realm, true);
            match realm.eval(Script::new(
                "test_tostring.js",
                r#"
                    let t = new com.company.Test();
                    t.foo = "bar";
                    com.company.Test.sfoo = "sbar"
                    t.foo + "_" + com.company.Test.sfoo;
                    "#,
            )) {
                Ok(res) => {
                    let s = res.to_string().expect("could not to_str");
                    assert_eq!(s, "bar_sbar");
                }
                Err(e) => {
                    panic!("e: {}", e);
                }
            }
        });
    }

    #[test]
    pub fn test_instance_of() {
        log::info!("> test_instance_of");

        let rt = init_test_rt();
        rt.exe_rt_task_in_event_loop(|q_js_rt| {
            q_js_rt.gc();
            let q_ctx = q_js_rt.get_main_realm();
            let _ = Proxy::new()
                .constructor(|_rt, _q_ctx, _id, _args| Ok(()))
                .namespace(&["com", "company"])
                .name("Test")
                .install(q_ctx, true);
            match q_ctx.eval(Script::new(
                "test_tostring.js",
                r#"
                    let t = new com.company.Test();
                    t instanceof com.company.Test
                    "#,
            )) {
                Ok(res) => {
                    let bln = res.to_bool();
                    assert!(bln);
                }
                Err(e) => {
                    panic!("e: {}", e);
                }
            }
        });
    }

    #[test]
    pub fn test_to_string() {
        log::info!("> test_proxy");

        let rt = init_test_rt();
        rt.exe_rt_task_in_event_loop(|q_js_rt| {
            q_js_rt.gc();
            let q_ctx = q_js_rt.get_main_realm();
            let _ = Proxy::new()
                .constructor(|_rt, _q_ctx, _id, _args| Ok(()))
                .namespace(&["com", "company"])
                .name("Test")
                .install(q_ctx, true);
            let res = q_ctx
                .eval(Script::new(
                    "test_tostring.es",
                    "com.company.Test + '-' + new com.company.Test()",
                ))
                .expect("script failed");
            let str = primitives::to_string_q(q_ctx, &res).expect("could not tostring");
            assert!(str.starts_with("Proxy::com.company.Test-Proxy::instance("));
            assert!(str.ends_with(")::com.company.Test"));
        });
    }

    #[test]
    pub fn test_proxy() {
        log::info!("> test_proxy");

        let rt = init_test_rt();
        rt.exe_rt_task_in_event_loop(|q_js_rt| {
            let q_ctx = q_js_rt.get_main_realm();
            let res = Proxy::new()
                .name("TestClass1")
                .constructor(|_rt, _context, id, _args| {
                    TEST_INSTANCES.with(|rc| {
                        let map = &mut *rc.borrow_mut();
                        map.insert(id, "hi".to_string())
                    });
                    Ok(())
                })
                .method("doIt", |_rt, _context, _obj_id, _args| {
                    Ok(primitives::from_i32(531))
                })
                .method("doIt2", |_rt, _context, _obj_id, _args| {
                    Err(JsError::new_str("aaargh"))
                })
                .getter_setter(
                    "gVar",
                    |_rt, _context, _id| Ok(primitives::from_i32(147)),
                    |_rt, _context, _id, _val| Ok(()),
                )
                .static_method("sDoIt", |_rt, _context, _args| {
                    Ok(primitives::from_i32(9876))
                })
                .static_method("sDoIt2", |_rt, _context, _args| {
                    Ok(primitives::from_i32(140))
                })
                .static_getter_setter(
                    "someThing",
                    |_rt, _context| {
                        trace!("static getter called, returning 754");
                        Ok(primitives::from_i32(754))
                    },
                    |_rt, q_ctx, val| {
                        trace!(
                            "static setter called, set to {}",
                            functions::call_to_string_q(q_ctx, &val)?
                        );
                        Ok(())
                    },
                )
                .finalizer(|_rt, _context, id| {
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

        let i2_res = rt.eval_sync(None, Script::new(
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

        let i = rt.eval_sync(None, Script::new(
            "test_proxy2.es",
            "let tc1 = new TestClass1(1, true, 'abc'); let r = tc1.doIt(1, true, 'abc'); r = tc1.doIt(1, true, 'abc'); tc1 = null; r;"
        ))
            .ok()
            .expect("script failed");

        assert!(i.is_i32());
        assert_eq!(i.get_i32(), 531);

        let i3_res = rt.eval_sync(None, Script::new("test_proxy.es", "TestClass1.sDoIt();"));

        if i3_res.is_err() {
            panic!("script failed: {}", i3_res.err().unwrap());
        }
        let i3 = i3_res.ok().unwrap();

        assert!(i3.is_i32());
        assert_eq!(i3.get_i32(), 9876);

        let i4 = rt
            .eval_sync(
                None,
                Script::new(
                    "test_proxy.es",
                    "TestClass1.someThing = 1; TestClass1.someThing;",
                ),
            )
            .expect("script failed");

        assert!(i4.is_i32());
        assert_eq!(i4.get_i32(), 754);

        let i5 = rt
            .eval_sync(
                None,
                Script::new(
                    "test_proxy.es",
                    "let tc5 = new TestClass1(); let r5 = tc5.gVar; tc5 = null; r5;",
                ),
            )
            .expect("script failed");

        assert!(i5.is_i32());
        assert_eq!(i5.get_i32(), 147);

        let i6_res = rt.eval_sync(
            None,
            Script::new(
                "test_proxy.es",
                "let tc6 = new TestClass1(); let r6 = tc6.doIt2(); tc6 = null; r6;",
            ),
        );
        assert!(i6_res.is_err());
        let e = i6_res.err().unwrap();
        let e_msg = e.get_message();
        assert_eq!(e_msg, "proxy_instance_method failed: aaargh");

        assert!(e.get_stack().contains("[doIt2]"));

        rt.gc_sync();

        std::thread::sleep(Duration::from_secs(1));

        log::info!("< test_proxy");
    }

    #[test]
    pub fn test_constructor() {
        // todo init logger

        let rt = init_test_rt();
        rt.loop_realm_sync(None, |_rt, realm| {
            Proxy::new()
                .name("TestClass")
                .namespace(&["com", "company"])
                .constructor(|_rt, _realm, _id, _args| Ok(()))
                .finalizer(|_rt, _realm, _id| {
                    //
                })
                .install(realm, true)
                .expect("poof");

            let constr_str = realm
                .eval(Script::new(
                    "test_constr.js",
                    r#"
                let instance = new com.company.TestClass();
                const ret = "" + instance.constructor.name;
                instance = null;
                ret
            "#,
                ))
                .expect("script failed");

            println!(
                "cons str = {}",
                constr_str.to_string().expect("not a string")
            );
        });
    }
}
