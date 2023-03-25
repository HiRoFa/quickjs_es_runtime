//! Utils for working with objects

use crate::quickjs_utils::properties::JSPropertyEnumRef;
use crate::quickjs_utils::{atoms, functions, get_constructor, get_global};
use crate::quickjsrealmadapter::QuickJsRealmAdapter;
use crate::quickjsruntimeadapter::{make_cstring, QuickJsRuntimeAdapter};
use crate::valueref::JSValueRef;
use hirofa_utils::js_utils::JsError;
use libquickjs_sys as q;

/// get a namespace object
/// this is used to get nested object properties which are used as namespaces
/// # Example
/// ```rust
/// use quickjs_runtime::builder::QuickJsRuntimeBuilder;
/// use quickjs_runtime::quickjs_utils::objects::get_namespace_q;
/// let rt = QuickJsRuntimeBuilder::new().build();
/// rt.exe_rt_task_in_event_loop(|q_js_rt| {
///     let q_ctx = q_js_rt.get_main_context();
///     let ns_obj = get_namespace_q(q_ctx, vec!["com", "hirofa", "examplepackage"], true).ok().unwrap();
///     assert!(ns_obj.is_object())
/// })
/// ```
pub fn get_namespace_q(
    context: &QuickJsRealmAdapter,
    namespace: Vec<&str>,
    create_if_absent: bool,
) -> Result<JSValueRef, JsError> {
    unsafe { get_namespace(context.context, namespace, create_if_absent) }
}

/// # Safety
/// when passing a context ptr please be sure that the corresponding QuickJsContext is still active
pub unsafe fn get_namespace(
    context: *mut q::JSContext,
    namespace: Vec<&str>,
    create_if_absent: bool,
) -> Result<JSValueRef, JsError> {
    log::trace!("objects::get_namespace({})", namespace.join("."));

    let mut obj = get_global(context);
    for p_name in namespace {
        log::trace!("objects::get_namespace -> {}", p_name);
        let mut sub = get_property(context, &obj, p_name)?;
        if sub.is_null_or_undefined() {
            log::trace!("objects::get_namespace -> is null");
            if create_if_absent {
                log::trace!("objects::get_namespace -> is null, creating");
                // create
                sub = create_object(context)?;
                set_property2(context, &obj, p_name, &sub, 0)?;
            } else {
                log::trace!("objects::get_namespace -> is null -> err");
                return Err(JsError::new_string(format!(
                    "could not find namespace part: {p_name}"
                )));
            }
        } else {
            log::trace!("objects::get_namespace -> found");
        }
        obj = sub;
    }

    Ok(obj)
}

#[allow(dead_code)]
/// construct a new instance of a constructor
/// # Safety
/// please ensure the passed JSContext is still valid
pub unsafe fn construct_object(
    ctx: *mut q::JSContext,
    constructor_ref: &JSValueRef,
    args: &[&JSValueRef],
) -> Result<JSValueRef, JsError> {
    let arg_count = args.len() as i32;

    let mut qargs = args.iter().map(|a| *a.borrow_value()).collect::<Vec<_>>();

    let res = q::JS_CallConstructor(
        ctx,
        *constructor_ref.borrow_value(),
        arg_count,
        qargs.as_mut_ptr(),
    );

    let res_ref = JSValueRef::new(ctx, res, false, true, "construct_object result");

    if res_ref.is_exception() {
        if let Some(ex) = QuickJsRealmAdapter::get_exception(ctx) {
            Err(ex)
        } else {
            Err(JsError::new_str(
                "construct_object failed but could not get ex",
            ))
        }
    } else {
        Ok(res_ref)
    }
}

/// create a new simple object, e.g. `let obj = {};`
pub fn create_object_q(q_ctx: &QuickJsRealmAdapter) -> Result<JSValueRef, JsError> {
    unsafe { create_object(q_ctx.context) }
}

/// create a new simple object, e.g. `let obj = {};`
/// # Safety
/// when passing a context ptr please be sure that the corresponding QuickJsContext is still active
pub unsafe fn create_object(context: *mut q::JSContext) -> Result<JSValueRef, JsError> {
    let obj = q::JS_NewObject(context);
    let obj_ref = JSValueRef::new(context, obj, false, true, "objects::create_object");
    if obj_ref.is_exception() {
        return Err(JsError::new_str("Could not create object"));
    }
    Ok(obj_ref)
}

/// set a property in an object, like `obj[propName] = val;`
pub fn set_property_q(
    q_ctx: &QuickJsRealmAdapter,
    obj_ref: &JSValueRef,
    prop_name: &str,
    prop_ref: &JSValueRef,
) -> Result<(), JsError> {
    unsafe { set_property(q_ctx.context, obj_ref, prop_name, prop_ref) }
}

/// set a property in an object, like `obj[propName] = val;`
/// # Safety
/// when passing a context ptr please be sure that the corresponding QuickJsContext is still active
pub unsafe fn set_property(
    context: *mut q::JSContext,
    obj_ref: &JSValueRef,
    prop_name: &str,
    prop_ref: &JSValueRef,
) -> Result<(), JsError> {
    set_property2(
        context,
        obj_ref,
        prop_name,
        prop_ref,
        q::JS_PROP_C_W_E as i32,
    )
}

/// set a property with specific flags
/// set_property applies the default flag JS_PROP_C_W_E (Configurable, Writable, Enumerable)
/// flags you can use here are
/// * q::JS_PROP_CONFIGURABLE
/// * q::JS_PROP_WRITABLE
/// * q::JS_PROP_ENUMERABLE          
/// * q::JS_PROP_C_W_E         
/// * q::JS_PROP_LENGTH        
/// * q::JS_PROP_TMASK         
/// * q::JS_PROP_NORMAL         
/// * q::JS_PROP_GETSET         
/// * q::JS_PROP_VARREF
/// * q::JS_PROP_AUTOINIT
/// # Example
/// ```rust
/// use quickjs_runtime::builder::QuickJsRuntimeBuilder;
/// use quickjs_runtime::quickjs_utils::objects::{create_object_q, set_property2_q};
/// use quickjs_runtime::quickjs_utils::primitives::from_i32;
/// use libquickjs_sys as q;
/// let rt = QuickJsRuntimeBuilder::new().build();
/// rt.exe_rt_task_in_event_loop(|q_js_rt| {
///    let q_ctx = q_js_rt.get_main_context();
///    let obj = create_object_q(q_ctx).ok().unwrap();
///    let prop = from_i32(785);
///    // not enumerable
///    set_property2_q(q_ctx, &obj, "someProp", &prop, (q::JS_PROP_CONFIGURABLE | q::JS_PROP_WRITABLE) as i32).ok().unwrap();
/// })
/// ```                         
pub fn set_property2_q(
    q_ctx: &QuickJsRealmAdapter,
    obj_ref: &JSValueRef,
    prop_name: &str,
    prop_ref: &JSValueRef,
    flags: i32,
) -> Result<(), JsError> {
    unsafe { set_property2(q_ctx.context, obj_ref, prop_name, prop_ref, flags) }
}

/// set a property with specific flags
/// # Safety
/// When passing a context pointer please make sure the corresponding QuickJsContext is still valid
pub unsafe fn set_property2(
    context: *mut q::JSContext,
    obj_ref: &JSValueRef,
    prop_name: &str,
    prop_ref: &JSValueRef,
    flags: i32,
) -> Result<(), JsError> {
    log::trace!("set_property2: {}", prop_name);

    let ckey = make_cstring(prop_name)?;

    /*
        pub const JS_PROP_CONFIGURABLE: u32 = 1;
    pub const JS_PROP_WRITABLE: u32 = 2;
    pub const JS_PROP_ENUMERABLE: u32 = 4;
    pub const JS_PROP_C_W_E: u32 = 7;
    pub const JS_PROP_LENGTH: u32 = 8;
    pub const JS_PROP_TMASK: u32 = 48;
    pub const JS_PROP_NORMAL: u32 = 0;
    pub const JS_PROP_GETSET: u32 = 16;
    pub const JS_PROP_VARREF: u32 = 32;
    pub const JS_PROP_AUTOINIT: u32 = 48;
        */

    log::trace!("set_property2 / 2");

    let ret = q::JS_DefinePropertyValueStr(
        context,
        *obj_ref.borrow_value(),
        ckey.as_ptr(),
        prop_ref.clone_value_incr_rc(),
        flags,
    );
    log::trace!("set_property2 / 3");
    if ret < 0 {
        return Err(JsError::new_str("Could not add property to object"));
    }
    log::trace!("set_property2 / 4");
    Ok(())
}

/// define a getter/setter property
/// # Example
/// ```dontrun
/// use quickjs_runtime::esruntimebuilder::EsRuntimeBuilder;
/// use quickjs_runtime::quickjs_utils::objects::{create_object_q, define_getter_setter_q, set_property_q};
/// use quickjs_runtime::quickjs_utils::functions::new_function_q;
/// use quickjs_runtime::quickjs_utils::primitives::from_i32;
/// use quickjs_runtime::quickjs_utils::{new_null_ref, get_global_q};
/// use hirofa_utils::js_utils::Script;
/// use quickjs_runtime::JsError::JsError;
/// let rt = EsRuntimeBuilder::new().build();
/// rt.add_to_event_queue_sync(|q_js_rt| {
///     let q_ctx = q_js_rt.get_main_context();
///     let obj = create_object_q(q_ctx).ok().expect("create obj failed");
///     let getter_func = new_function_q(q_ctx, "getter", |_q_ctx, _this_ref, _args| {Ok(from_i32(13))}, 0).ok().expect("new_function_q getter failed");
///     let setter_func = new_function_q(q_ctx, "setter", |_q_ctx, _this_ref, args| {
///         log::debug!("setting someProperty to {:?}", &args[0]);
///         Ok(new_null_ref())
///     }, 1).ok().expect("new_function_q setter failed");
///     let res = define_getter_setter_q(q_ctx, &obj, "someProperty", &getter_func, &setter_func);
///     match res {
///         Ok(_) => {},
///         Err(e) => {panic!("define_getter_setter_q fail: {}", e)}}
///     let global = get_global_q(q_ctx);
///     set_property_q(q_ctx, &global, "testObj431", &obj).ok().expect("set prop on global failed");
/// });
/// rt.eval_sync(Script::new("define_getter_setter_q.es", "testObj431.someProperty = 'hello prop';")).ok().expect("script failed");
/// ```
pub fn define_getter_setter_q(
    q_ctx: &QuickJsRealmAdapter,
    obj_ref: &JSValueRef,
    prop_name: &str,
    getter_func_ref: &JSValueRef,
    setter_func_ref: &JSValueRef,
) -> Result<(), JsError> {
    unsafe {
        define_getter_setter(
            q_ctx.context,
            obj_ref,
            prop_name,
            getter_func_ref,
            setter_func_ref,
        )
    }
}

#[allow(dead_code)]
/// define a getter/setter property
/// # Safety
/// When passing a context pointer please make sure the corresponding QuickJsContext is still valid
pub unsafe fn define_getter_setter(
    context: *mut q::JSContext,
    obj_ref: &JSValueRef,
    prop_name: &str,
    getter_func_ref: &JSValueRef,
    setter_func_ref: &JSValueRef,
) -> Result<(), JsError> {
    /*
     pub fn JS_DefinePropertyGetSet(
        ctx: *mut JSContext,
        this_obj: JSValue,
        prop: JSAtom,
        getter: JSValue,
        setter: JSValue,
        flags: ::std::os::raw::c_int,
    ) -> ::std::os::raw::c_int;
     */

    log::trace!("objects::define_getter_setter 1");

    debug_assert!(functions::is_function(context, getter_func_ref));
    log::trace!("objects::define_getter_setter 2");
    debug_assert!(functions::is_function(context, setter_func_ref));
    log::trace!("objects::define_getter_setter 3");

    let prop_atom = atoms::from_string(context, prop_name)?;

    log::trace!("objects::define_getter_setter 4");

    let res = q::JS_DefinePropertyGetSet(
        context,
        *obj_ref.borrow_value(),
        prop_atom.get_atom(),
        getter_func_ref.clone_value_incr_rc(),
        setter_func_ref.clone_value_incr_rc(),
        q::JS_PROP_C_W_E as i32,
    );

    log::trace!("objects::define_getter_setter 5 {}", res);

    if res != 0 {
        if let Some(err) = QuickJsRealmAdapter::get_exception(context) {
            Err(err)
        } else {
            Err(JsError::new_str(
                "Unknown error while creating getter setter",
            ))
        }
    } else {
        Ok(())
    }
}

/// get a property from an object by name
pub fn get_property_q(
    q_ctx: &QuickJsRealmAdapter,
    obj_ref: &JSValueRef,
    prop_name: &str,
) -> Result<JSValueRef, JsError> {
    unsafe { get_property(q_ctx.context, obj_ref, prop_name) }
}

/// get a property from an object by name
/// # Safety
/// when passing a context please ensure the corresponding QuickJsContext is still valid
pub unsafe fn get_property(
    context: *mut q::JSContext,
    obj_ref: &JSValueRef,
    prop_name: &str,
) -> Result<JSValueRef, JsError> {
    if obj_ref.is_null() || obj_ref.is_undefined() {
        return Err(JsError::new_str(
            "could not get prop from null or undefined",
        ));
    }

    let c_prop_name = make_cstring(prop_name)?;

    log::trace!("objects::get_property {}", prop_name);

    let prop_val = q::JS_GetPropertyStr(context, *obj_ref.borrow_value(), c_prop_name.as_ptr());
    let prop_ref = JSValueRef::new(
        context,
        prop_val,
        false,
        true,
        format!("object::get_property result: {prop_name}").as_str(),
    );

    Ok(prop_ref)
}

/// get the property names of an object
pub fn get_own_property_names_q(
    q_ctx: &QuickJsRealmAdapter,
    obj_ref: &JSValueRef,
) -> Result<JSPropertyEnumRef, JsError> {
    unsafe { get_own_property_names(q_ctx.context, obj_ref) }
}

/// get the property names of an object
/// # Safety
/// When passing a context pointer please make sure the corresponding QuickJsContext is still valid
pub unsafe fn get_own_property_names(
    context: *mut q::JSContext,
    obj_ref: &JSValueRef,
) -> Result<JSPropertyEnumRef, JsError> {
    let mut properties: *mut q::JSPropertyEnum = std::ptr::null_mut();
    let mut count: u32 = 0;

    let flags = (q::JS_GPN_STRING_MASK | q::JS_GPN_SYMBOL_MASK | q::JS_GPN_ENUM_ONLY) as i32;
    let ret = q::JS_GetOwnPropertyNames(
        context,
        &mut properties,
        &mut count,
        *obj_ref.borrow_value(),
        flags,
    );
    if ret != 0 {
        return Err(JsError::new_str("Could not get object properties"));
    }

    let enum_ref = JSPropertyEnumRef::new(context, properties, count);
    Ok(enum_ref)
}

/// get the names of all properties of an object
pub fn get_property_names_q(
    q_ctx: &QuickJsRealmAdapter,
    obj_ref: &JSValueRef,
) -> Result<Vec<String>, JsError> {
    unsafe { get_property_names(q_ctx.context, obj_ref) }
}

/// get the names of all properties of an object
/// # Safety
/// When passing a context pointer please make sure the corresponding QuickJsContext is still valid
pub unsafe fn get_property_names(
    context: *mut q::JSContext,
    obj_ref: &JSValueRef,
) -> Result<Vec<String>, JsError> {
    let enum_ref = get_own_property_names(context, obj_ref)?;

    let mut names = vec![];

    for index in 0..enum_ref.len() {
        let name = enum_ref.get_name(index)?;
        names.push(name);
    }

    Ok(names)
}

pub fn traverse_properties_q<V, R>(
    q_ctx: &QuickJsRealmAdapter,
    obj_ref: &JSValueRef,
    visitor: V,
) -> Result<Vec<R>, JsError>
where
    V: Fn(&str, &JSValueRef) -> Result<R, JsError>,
{
    unsafe { traverse_properties(q_ctx.context, obj_ref, visitor) }
}

pub fn traverse_properties_q_mut<V, R>(
    q_ctx: &QuickJsRealmAdapter,
    obj_ref: &JSValueRef,
    visitor: V,
) -> Result<(), JsError>
where
    V: FnMut(&str, &JSValueRef) -> Result<R, JsError>,
{
    unsafe { traverse_properties_mut(q_ctx.context, obj_ref, visitor) }
}

/// # Safety
/// When passing a context pointer please make sure the corresponding QuickJsContext is still valid
pub unsafe fn traverse_properties<V, R>(
    context: *mut q::JSContext,
    obj_ref: &JSValueRef,
    visitor: V,
) -> Result<Vec<R>, JsError>
where
    V: Fn(&str, &JSValueRef) -> Result<R, JsError>,
{
    let enum_ref = get_own_property_names(context, obj_ref)?;

    let mut result = vec![];

    for index in 0..enum_ref.len() {
        let atom = enum_ref.get_atom_raw(index) as q::JSAtom;
        let prop_name = atoms::to_str(context, &atom)?;

        let raw_value = q::JS_GetPropertyInternal(
            context,
            *obj_ref.borrow_value(),
            atom,
            *obj_ref.borrow_value(),
            0,
        );
        let prop_val_ref = JSValueRef::new(
            context,
            raw_value,
            false,
            true,
            "objects::traverse_properties raw_value",
        );
        if prop_val_ref.is_exception() {
            return Err(JsError::new_str("Could not get object property"));
        }

        let r = visitor(prop_name, &prop_val_ref)?;

        result.push(r);
    }

    Ok(result)
}

/// # Safety
/// When passing a context pointer please make sure the corresponding QuickJsContext is still valid
pub unsafe fn traverse_properties_mut<V, R>(
    context: *mut q::JSContext,
    obj_ref: &JSValueRef,
    mut visitor: V,
) -> Result<(), JsError>
where
    V: FnMut(&str, &JSValueRef) -> Result<R, JsError>,
{
    let enum_ref = get_own_property_names(context, obj_ref)?;

    for index in 0..enum_ref.len() {
        let atom = enum_ref.get_atom_raw(index) as q::JSAtom;
        let prop_name = atoms::to_str(context, &atom)?;

        let raw_value = q::JS_GetPropertyInternal(
            context,
            *obj_ref.borrow_value(),
            atom,
            *obj_ref.borrow_value(),
            0,
        );
        let prop_val_ref = JSValueRef::new(
            context,
            raw_value,
            false,
            true,
            "objects::traverse_properties raw_value",
        );
        if prop_val_ref.is_exception() {
            return Err(JsError::new_str("Could not get object property"));
        }

        visitor(prop_name, &prop_val_ref)?;
    }

    Ok(())
}

pub fn get_prototype_of_q(
    q_ctx: &QuickJsRealmAdapter,
    obj_ref: &JSValueRef,
) -> Result<JSValueRef, JsError> {
    unsafe { get_prototype_of(q_ctx.context, obj_ref) }
}

/// Object.prototypeOf
/// # Safety
/// please ensure the JSContext is valid and remains valid while using this function
pub unsafe fn get_prototype_of(
    ctx: *mut q::JSContext,
    obj_ref: &JSValueRef,
) -> Result<JSValueRef, JsError> {
    let raw = q::JS_GetPrototype(ctx, *obj_ref.borrow_value());
    let pt_ref = JSValueRef::new(ctx, raw, false, true, "object::get_prototype_of_q");

    if pt_ref.is_exception() {
        if let Some(ex) = QuickJsRealmAdapter::get_exception(ctx) {
            Err(ex)
        } else {
            Err(JsError::new_str(
                "get_prototype_of_q failed but could not get ex",
            ))
        }
    } else {
        Ok(pt_ref)
    }
}

pub fn is_instance_of_q(
    q_ctx: &QuickJsRealmAdapter,
    obj_ref: &JSValueRef,
    constructor_ref: &JSValueRef,
) -> bool {
    unsafe { is_instance_of(q_ctx.context, obj_ref, constructor_ref) }
}

/// # Safety
/// When passing a context pointer please make sure the corresponding QuickJsContext is still valid
pub unsafe fn is_instance_of(
    context: *mut q::JSContext,
    obj_ref: &JSValueRef,
    constructor_ref: &JSValueRef,
) -> bool {
    if !obj_ref.is_object() {
        return false;
    }

    q::JS_IsInstanceOf(
        context,
        *obj_ref.borrow_value(),
        *constructor_ref.borrow_value(),
    ) > 0
}

pub fn is_instance_of_by_name_q(
    context: &QuickJsRealmAdapter,
    obj_ref: &JSValueRef,
    constructor_name: &str,
) -> Result<bool, JsError> {
    unsafe { is_instance_of_by_name(context.context, obj_ref, constructor_name) }
}

/// # Safety
/// When passing a context pointer please make sure the corresponding QuickJsContext is still valid
pub unsafe fn is_instance_of_by_name(
    context: *mut q::JSContext,
    obj_ref: &JSValueRef,
    constructor_name: &str,
) -> Result<bool, JsError> {
    if !obj_ref.is_object() {
        return Ok(false);
    }

    let constructor_ref = get_constructor(context, constructor_name)?;
    if !constructor_ref.is_object() {
        return Ok(false);
    }

    if is_instance_of(context, obj_ref, &constructor_ref) {
        Ok(true)
    } else {
        // todo check if context is not __main__
        QuickJsRuntimeAdapter::do_with(|q_js_rt| {
            let main_ctx = q_js_rt.get_main_context();
            let main_constructor_ref = get_constructor(main_ctx.context, constructor_name)?;
            if is_instance_of(main_ctx.context, obj_ref, &main_constructor_ref) {
                Ok(true)
            } else {
                Ok(false)
            }
        })
    }
}

#[cfg(test)]
pub mod tests {
    use crate::facades::tests::init_test_rt;
    use crate::quickjs_utils::objects::{
        create_object_q, get_property_names_q, get_property_q, set_property_q,
    };
    use crate::quickjs_utils::primitives::{from_i32, to_i32};
    use crate::quickjs_utils::{get_global_q, primitives};
    use hirofa_utils::js_utils::Script;

    #[test]
    fn test_get_refs() {
        let rt = init_test_rt();
        rt.exe_rt_task_in_event_loop(|q_js_rt| {
            let q_ctx = q_js_rt.get_main_context();
            let obj = create_object_q(q_ctx).ok().expect("a");
            let prop_ref = create_object_q(q_ctx).ok().expect("b");
            let prop2_ref = create_object_q(q_ctx).ok().expect("c");
            assert_eq!(obj.get_ref_count(), 1);
            assert_eq!(prop_ref.get_ref_count(), 1);
            set_property_q(q_ctx, &obj, "a", &prop_ref).ok().expect("d");
            assert_eq!(prop_ref.get_ref_count(), 2);
            set_property_q(q_ctx, &obj, "b", &prop_ref).ok().expect("e");
            assert_eq!(prop_ref.get_ref_count(), 3);
            set_property_q(q_ctx, &obj, "b", &prop2_ref)
                .ok()
                .expect("f");
            assert_eq!(prop_ref.get_ref_count(), 2);
            assert_eq!(prop2_ref.get_ref_count(), 2);

            let p3 = get_property_q(q_ctx, &obj, "b").ok().expect("g");
            assert_eq!(p3.get_ref_count(), 3);
            assert_eq!(prop2_ref.get_ref_count(), 3);

            drop(p3);
            assert_eq!(prop2_ref.get_ref_count(), 2);

            drop(obj);

            q_js_rt.gc();
        });
    }

    #[test]
    fn test_get_n_drop() {
        log::info!("> test_get_n_drop");

        let rt = init_test_rt();
        rt.exe_rt_task_in_event_loop(|q_js_rt| {
            let q_ctx = q_js_rt.get_main_context();

            let obj_a = create_object_q(q_ctx).ok().unwrap();
            let obj_b = create_object_q(q_ctx).ok().unwrap();
            set_property_q(q_ctx, &obj_a, "b", &obj_b).ok().unwrap();

            let b1 = get_property_q(q_ctx, &obj_a, "b").ok().unwrap();
            set_property_q(q_ctx, &b1, "i", &primitives::from_i32(123))
                .ok()
                .unwrap();
            drop(b1);
            q_js_rt.gc();
            let b2 = get_property_q(q_ctx, &obj_a, "b").ok().unwrap();
            let i_ref = get_property_q(q_ctx, &b2, "i").ok().unwrap();
            let i = to_i32(&i_ref).ok().unwrap();
            drop(i_ref);
            q_js_rt.gc();
            let i_ref2 = get_property_q(q_ctx, &b2, "i").ok().unwrap();
            let i2 = to_i32(&i_ref2).ok().unwrap();

            assert_eq!(i, 123);
            assert_eq!(i2, 123);
        });

        log::info!("< test_get_n_drop");
    }

    #[test]
    fn test_propnames() {
        log::info!("> test_propnames");

        let rt = init_test_rt();
        let io = rt.exe_rt_task_in_event_loop(|q_js_rt| {
            let q_ctx = q_js_rt.get_main_context();

            let obj_ref = q_ctx
                .eval(Script::new("test_propnames.es", "({one: 1, two: 2});"))
                .ok()
                .expect("could not get test obj");
            let prop_names = get_property_names_q(q_ctx, &obj_ref)
                .ok()
                .expect("could not get prop names");

            assert_eq!(prop_names.len(), 2);

            assert!(prop_names.contains(&"one".to_string()));
            assert!(prop_names.contains(&"two".to_string()));
            true
        });
        assert!(io);

        log::info!("< test_propnames");
    }

    #[test]
    fn test_set_prop() {
        log::info!("> test_set_prop");

        let rt = init_test_rt();
        let io = rt.exe_rt_task_in_event_loop(|q_js_rt| {
            let q_ctx = q_js_rt.get_main_context();

            let obj_ref = create_object_q(q_ctx).ok().unwrap();

            assert_eq!(obj_ref.get_ref_count(), 1);

            let global_ref = get_global_q(q_ctx);
            set_property_q(q_ctx, &global_ref, "test_obj", &obj_ref)
                .ok()
                .expect("could not set property 1");

            assert_eq!(obj_ref.get_ref_count(), 2);

            let prop_ref = from_i32(123);
            let obj_ref = get_property_q(q_ctx, &global_ref, "test_obj")
                .ok()
                .expect("could not get test_obj");
            set_property_q(q_ctx, &obj_ref, "test_prop", &prop_ref)
                .ok()
                .expect("could not set property 2");

            drop(global_ref);

            q_js_rt.gc();

            let a = q_ctx
                .eval(Script::new("test_set_prop.es", "(test_obj);"))
                .ok()
                .unwrap()
                .is_object();
            assert!(a);
            let b = q_ctx
                .eval(Script::new("test_set_prop.es", "(test_obj.test_prop);"))
                .ok()
                .unwrap()
                .is_i32();
            assert!(b);
            a && b
        });
        assert!(io);

        log::info!("< test_set_prop");
    }
}
