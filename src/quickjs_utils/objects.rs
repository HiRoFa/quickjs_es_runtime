use crate::droppable_value::DroppableValue;
use crate::eserror::EsError;
use crate::quickjs_utils::{atoms, functions, get_constructor, get_global};
use crate::quickjscontext::QuickJsContext;
use crate::quickjsruntime::{make_cstring, QuickJsRuntime};
use crate::valueref::JSValueRef;
use libquickjs_sys as q;
use std::collections::HashMap;

pub fn get_namespace_q(
    context: &QuickJsContext,
    namespace: Vec<&str>,
    create_if_absent: bool,
) -> Result<JSValueRef, EsError> {
    unsafe { get_namespace(context.context, namespace, create_if_absent) }
}

/// # Safety
/// when passing a context ptr please be sure that the corresponding QuickJsContext is still active
/// # Safety
/// When passing a context pointer please make sure the corresponding QuickJsContext is still valid
pub unsafe fn get_namespace(
    context: *mut q::JSContext,
    namespace: Vec<&str>,
    create_if_absent: bool,
) -> Result<JSValueRef, EsError> {
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
                return Err(EsError::new_string(format!(
                    "could not find namespace part: {}",
                    p_name
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
    args: Vec<JSValueRef>,
) -> Result<JSValueRef, EsError> {
    let arg_count = args.len() as i32;

    let mut qargs = args.iter().map(|a| *a.borrow_value()).collect::<Vec<_>>();

    let res = q::JS_CallConstructor(
        ctx,
        *constructor_ref.borrow_value(),
        arg_count,
        qargs.as_mut_ptr(),
    );

    let res_ref = JSValueRef::new(ctx, res, false, true, "call_function result");

    if res_ref.is_exception() {
        if let Some(ex) = QuickJsContext::get_exception(ctx) {
            Err(ex)
        } else {
            Err(EsError::new_str(
                "construct_object failed but could not get ex",
            ))
        }
    } else {
        Ok(res_ref)
    }
}

/// create a new simple object, e.g. `let obj = {};`
pub fn create_object_q(q_ctx: &QuickJsContext) -> Result<JSValueRef, EsError> {
    unsafe { create_object(q_ctx.context) }
}

/// create a new simple object, e.g. `let obj = {};`
/// # Safety
/// when passing a context ptr please be sure that the corresponding QuickJsContext is still active
pub unsafe fn create_object(context: *mut q::JSContext) -> Result<JSValueRef, EsError> {
    let obj = q::JS_NewObject(context);
    let obj_ref = JSValueRef::new(context, obj, false, true, "objects::create_object");
    if obj_ref.is_exception() {
        return Err(EsError::new_str("Could not create object"));
    }
    Ok(obj_ref)
}

/// set a property in an object, like `obj[propName] = val;`
pub fn set_property_q(
    q_ctx: &QuickJsContext,
    obj_ref: &JSValueRef,
    prop_name: &str,
    prop_ref: &JSValueRef,
) -> Result<(), EsError> {
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
) -> Result<(), EsError> {
    set_property2(
        context,
        obj_ref,
        prop_name,
        prop_ref,
        q::JS_PROP_C_W_E as i32,
    )
}

pub fn set_property2_q(
    q_ctx: &QuickJsContext,
    obj_ref: &JSValueRef,
    prop_name: &str,
    prop_ref: &JSValueRef,
    flags: i32,
) -> Result<(), EsError> {
    unsafe { set_property2(q_ctx.context, obj_ref, prop_name, prop_ref, flags) }
}

/// # Safety
/// When passing a context pointer please make sure the corresponding QuickJsContext is still valid
pub unsafe fn set_property2(
    context: *mut q::JSContext,
    obj_ref: &JSValueRef,
    prop_name: &str,
    prop_ref: &JSValueRef,
    flags: i32,
) -> Result<(), EsError> {
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
        return Err(EsError::new_str("Could not add property to object"));
    }
    log::trace!("set_property2 / 4");
    Ok(())
}

#[allow(dead_code)]
/// # Safety
/// When passing a context pointer please make sure the corresponding QuickJsContext is still valid
pub unsafe fn define_getter_setter(
    context: *mut q::JSContext,
    obj_ref: &JSValueRef,
    prop_name: &str,
    getter_func_ref: &JSValueRef,
    setter_func_ref: &JSValueRef,
) -> Result<(), EsError> {
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

    debug_assert!(functions::is_function(context, &getter_func_ref));
    log::trace!("objects::define_getter_setter 2");
    debug_assert!(functions::is_function(context, &setter_func_ref));
    log::trace!("objects::define_getter_setter 3");

    let prop_atom = atoms::from_string(context, prop_name)?;

    log::trace!("objects::define_getter_setter 4");

    let res = q::JS_DefinePropertyGetSet(
        context,
        *obj_ref.borrow_value(),
        prop_atom.get_atom(),
        *getter_func_ref.borrow_value(),
        *setter_func_ref.borrow_value(),
        0,
    );

    log::trace!("objects::define_getter_setter 5 {}", res);

    if res != 0 {
        if let Some(err) = QuickJsContext::get_exception(context) {
            Err(err)
        } else {
            Err(EsError::new_str(
                "Unknown error while creating getter setter",
            ))
        }
    } else {
        Ok(())
    }
}

/// get a property from an object by name
pub fn get_property_q(
    q_ctx: &QuickJsContext,
    obj_ref: &JSValueRef,
    prop_name: &str,
) -> Result<JSValueRef, EsError> {
    unsafe { get_property(q_ctx.context, obj_ref, prop_name) }
}

/// get a property from an object by name
/// # Safety
/// when passing a context please ensure the corresponding QuickJsContext is still valid
pub unsafe fn get_property(
    context: *mut q::JSContext,
    obj_ref: &JSValueRef,
    prop_name: &str,
) -> Result<JSValueRef, EsError> {
    if obj_ref.is_null() || obj_ref.is_undefined() {
        return Err(EsError::new_str(
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
        format!("object::get_property result: {}", prop_name).as_str(),
    );

    Ok(prop_ref)
}

pub fn get_property_names_q(
    q_ctx: &QuickJsContext,
    obj_ref: &JSValueRef,
) -> Result<Vec<String>, EsError> {
    unsafe { get_property_names(q_ctx.context, obj_ref) }
}

/// # Safety
/// When passing a context pointer please make sure the corresponding QuickJsContext is still valid
pub unsafe fn get_property_names(
    context: *mut q::JSContext,
    obj_ref: &JSValueRef,
) -> Result<Vec<String>, EsError> {
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
        return Err(EsError::new_str("Could not get object properties"));
    }

    let properties = DroppableValue::new(properties, |&mut properties| {
        for index in 0..count {
            let prop = properties.offset(index as isize);

            q::JS_FreeAtom(context, (*prop).atom);
        }

        q::js_free(context, properties as *mut std::ffi::c_void);
    });

    let mut res: Vec<String> = vec![];
    for index in 0..count {
        let prop = (*properties).offset(index as isize);

        let key_value = q::JS_AtomToString(context, (*prop).atom);
        let key_ref = JSValueRef::new(
            context,
            key_value,
            false,
            true,
            "objects::get_property_names key_value",
        );
        if key_ref.is_exception() {
            return Err(EsError::new_str("Could not get object property name"));
        }

        let key_str = crate::quickjs_utils::primitives::to_string(context, &key_ref)?;
        res.push(key_str);
    }
    Ok(res)
}

pub fn traverse_properties_q<V, R>(
    q_ctx: &QuickJsContext,
    obj_ref: &JSValueRef,
    visitor: V,
) -> Result<HashMap<String, R>, EsError>
where
    V: Fn(&str, JSValueRef) -> Result<R, EsError>,
{
    unsafe { traverse_properties(q_ctx.context, obj_ref, visitor) }
}

/// # Safety
/// When passing a context pointer please make sure the corresponding QuickJsContext is still valid
pub unsafe fn traverse_properties<V, R>(
    context: *mut q::JSContext,
    obj_ref: &JSValueRef,
    visitor: V,
) -> Result<HashMap<String, R>, EsError>
where
    V: Fn(&str, JSValueRef) -> Result<R, EsError>,
{
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
        return Err(EsError::new_str("Could not get object properties"));
    }

    let properties = DroppableValue::new(properties, |&mut properties| {
        for index in 0..count {
            let prop = properties.offset(index as isize);

            q::JS_FreeAtom(context, (*prop).atom);
        }

        q::js_free(context, properties as *mut std::ffi::c_void);
    });

    let mut map = HashMap::new();

    for index in 0..count {
        let prop = (*properties).offset(index as isize);
        let raw_value = q::JS_GetPropertyInternal(
            context,
            *obj_ref.borrow_value(),
            (*prop).atom,
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
            return Err(EsError::new_str("Could not get object property"));
        }

        let key_value = q::JS_AtomToString(context, (*prop).atom);
        let key_ref = JSValueRef::new(
            context,
            key_value,
            false,
            true,
            "objects::traverse_properties key_value",
        );
        if key_ref.is_exception() {
            return Err(EsError::new_str("Could not get object property name"));
        }

        let key_str = crate::quickjs_utils::primitives::to_string(context, &key_ref)?;
        let r = visitor(key_str.as_str(), prop_val_ref)?;

        map.insert(key_str, r);
    }

    Ok(map)
}

pub fn is_instance_of_q(
    q_ctx: &QuickJsContext,
    obj_ref: &JSValueRef,
    constructor_ref: JSValueRef,
) -> bool {
    unsafe { is_instance_of(q_ctx.context, obj_ref, constructor_ref) }
}

/// # Safety
/// When passing a context pointer please make sure the corresponding QuickJsContext is still valid
pub unsafe fn is_instance_of(
    context: *mut q::JSContext,
    obj_ref: &JSValueRef,
    constructor_ref: JSValueRef,
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
    context: &QuickJsContext,
    obj_ref: &JSValueRef,
    constructor_name: &str,
) -> Result<bool, EsError> {
    unsafe { is_instance_of_by_name(context.context, obj_ref, constructor_name) }
}

/// # Safety
/// When passing a context pointer please make sure the corresponding QuickJsContext is still valid
pub unsafe fn is_instance_of_by_name(
    context: *mut q::JSContext,
    obj_ref: &JSValueRef,
    constructor_name: &str,
) -> Result<bool, EsError> {
    if !obj_ref.is_object() {
        return Ok(false);
    }

    let constructor_ref = get_constructor(context, constructor_name)?;
    if !constructor_ref.is_object() {
        return Ok(false);
    }

    if is_instance_of(context, obj_ref, constructor_ref) {
        Ok(true)
    } else {
        // todo check if context is not __main__
        QuickJsRuntime::do_with(|q_js_rt| {
            let main_ctx = q_js_rt.get_main_context();
            let main_constructor_ref = get_constructor(main_ctx.context, constructor_name)?;
            if is_instance_of(main_ctx.context, obj_ref, main_constructor_ref) {
                Ok(true)
            } else {
                Ok(false)
            }
        })
    }
}

#[cfg(test)]
pub mod tests {
    use crate::esruntime::EsRuntime;
    use crate::esscript::EsScript;
    use crate::quickjs_utils::objects::{
        create_object_q, get_property_names_q, get_property_q, set_property_q,
    };
    use crate::quickjs_utils::primitives::{from_i32, to_i32};
    use crate::quickjs_utils::{get_global_q, primitives};
    use std::sync::Arc;

    #[test]
    fn test_get_refs() {
        let rt: Arc<EsRuntime> = crate::esruntime::tests::TEST_ESRT.clone();
        rt.add_to_event_queue_sync(|q_js_rt| {
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

        let rt: Arc<EsRuntime> = crate::esruntime::tests::TEST_ESRT.clone();
        rt.add_to_event_queue_sync(|q_js_rt| {
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

        let rt: Arc<EsRuntime> = crate::esruntime::tests::TEST_ESRT.clone();
        let io = rt.add_to_event_queue_sync(|q_js_rt| {
            let q_ctx = q_js_rt.get_main_context();

            let obj_ref = q_ctx
                .eval(EsScript::new("test_propnames.es", "({one: 1, two: 2});"))
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

        let rt: Arc<EsRuntime> = crate::esruntime::tests::TEST_ESRT.clone();
        let io = rt.add_to_event_queue_sync(|q_js_rt| {
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
                .eval(EsScript::new("test_set_prop.es", "(test_obj);"))
                .ok()
                .unwrap()
                .is_object();
            assert!(a);
            let b = q_ctx
                .eval(EsScript::new("test_set_prop.es", "(test_obj.test_prop);"))
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
