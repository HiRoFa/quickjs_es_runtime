use crate::droppable_value::DroppableValue;
use crate::eserror::EsError;
use crate::quickjs_utils::{atoms, functions, get_constructor, get_global};
use crate::quickjsruntime::{make_cstring, QuickJsRuntime};
use crate::valueref::JSValueRef;
use libquickjs_sys as q;
use std::collections::HashMap;

pub fn get_namespace(
    q_js_rt: &QuickJsRuntime,
    namespace: Vec<&str>,
    create_if_absent: bool,
) -> Result<JSValueRef, EsError> {
    log::trace!("objects::get_namespace({})", namespace.join("."));

    let mut obj = get_global(q_js_rt);
    for p_name in namespace {
        log::trace!("objects::get_namespace -> {}", p_name);
        let mut sub = get_property(q_js_rt, &obj, p_name)?;
        if sub.is_null_or_undefined() {
            log::trace!("objects::get_namespace -> is null");
            if create_if_absent {
                log::trace!("objects::get_namespace -> is null, creating");
                // create
                sub = create_object(q_js_rt)?;
                set_property2(q_js_rt, &obj, p_name, sub.clone(), 0)?;
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
pub fn construct_object(
    _q_js_rt: &QuickJsRuntime,
    _constructor_ref: &JSValueRef,
) -> Result<JSValueRef, EsError> {
    /*
    q::JS_CallConstructor(
        context,
        date_constructor,
        args.len() as i32,
        args.as_mut_ptr(),
    )

     */
    unimplemented!();
}

pub fn create_object(q_js_rt: &QuickJsRuntime) -> Result<JSValueRef, EsError> {
    let obj = unsafe { q::JS_NewObject(q_js_rt.context) };
    let obj_ref = JSValueRef::new(obj, false, true, "objects::create_object");
    if obj_ref.is_exception() {
        return Err(EsError::new_str("Could not create object"));
    }
    Ok(obj_ref)
}
pub fn set_property(
    q_js_rt: &QuickJsRuntime,
    obj_ref: &JSValueRef,
    prop_name: &str,
    prop_ref: JSValueRef,
) -> Result<(), EsError> {
    set_property2(
        q_js_rt,
        obj_ref,
        prop_name,
        prop_ref,
        q::JS_PROP_C_W_E as i32,
    )
}

pub fn set_property2(
    q_js_rt: &QuickJsRuntime,
    obj_ref: &JSValueRef,
    prop_name: &str,
    prop_ref: JSValueRef,
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

    let ret = unsafe {
        q::JS_DefinePropertyValueStr(
            q_js_rt.context,
            *obj_ref.borrow_value(),
            ckey.as_ptr(),
            prop_ref.consume_value_no_decr_rc(),
            flags,
        )
    };
    log::trace!("set_property2 / 3");
    if ret < 0 {
        return Err(EsError::new_str("Could not add property to object"));
    }
    log::trace!("set_property2 / 4");
    Ok(())
}

#[allow(dead_code)]
pub fn define_getter_setter(
    q_js_rt: &QuickJsRuntime,
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

    assert!(functions::is_function(q_js_rt, &getter_func_ref));
    log::trace!("objects::define_getter_setter 2");
    assert!(functions::is_function(q_js_rt, &setter_func_ref));
    log::trace!("objects::define_getter_setter 3");

    let prop_atom = atoms::from_string(q_js_rt, prop_name)?;

    log::trace!("objects::define_getter_setter 4");

    let res = unsafe {
        q::JS_DefinePropertyGetSet(
            q_js_rt.context,
            *obj_ref.borrow_value(),
            prop_atom,
            *getter_func_ref.borrow_value(),
            *setter_func_ref.borrow_value(),
            0,
        )
    };

    log::trace!("objects::define_getter_setter 5 {}", res);

    if res != 0 {
        if let Some(err) = q_js_rt.get_exception() {
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

pub fn get_property(
    q_js_rt: &QuickJsRuntime,
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

    let prop_val = unsafe {
        q::JS_GetPropertyStr(
            q_js_rt.context,
            *obj_ref.borrow_value(),
            c_prop_name.as_ptr(),
        )
    };
    let prop_ref = JSValueRef::new(
        prop_val,
        false,
        true,
        format!("object::get_property result: {}", prop_name).as_str(),
    );

    Ok(prop_ref)
}

#[allow(dead_code)]
pub fn get_property_names(
    q_js_rt: &QuickJsRuntime,
    obj_ref: &JSValueRef,
) -> Result<Vec<String>, EsError> {
    let mut properties: *mut q::JSPropertyEnum = std::ptr::null_mut();
    let mut count: u32 = 0;

    let flags = (q::JS_GPN_STRING_MASK | q::JS_GPN_SYMBOL_MASK | q::JS_GPN_ENUM_ONLY) as i32;
    let ret = unsafe {
        q::JS_GetOwnPropertyNames(
            q_js_rt.context,
            &mut properties,
            &mut count,
            *obj_ref.borrow_value(),
            flags,
        )
    };
    if ret != 0 {
        return Err(EsError::new_str("Could not get object properties"));
    }

    let properties = DroppableValue::new(properties, |&mut properties| {
        for index in 0..count {
            let prop = unsafe { properties.offset(index as isize) };
            unsafe {
                q::JS_FreeAtom(q_js_rt.context, (*prop).atom);
            }
        }
        unsafe {
            q::js_free(q_js_rt.context, properties as *mut std::ffi::c_void);
        }
    });

    let mut res: Vec<String> = vec![];
    for index in 0..count {
        let prop = unsafe { (*properties).offset(index as isize) };

        let key_value = unsafe { q::JS_AtomToString(q_js_rt.context, (*prop).atom) };
        let key_ref = JSValueRef::new(
            key_value,
            false,
            true,
            "objects::get_property_names key_value",
        );
        if key_ref.is_exception() {
            return Err(EsError::new_str("Could not get object property name"));
        }

        let key_str = crate::quickjs_utils::primitives::to_string(q_js_rt, &key_ref)?;
        res.push(key_str);
    }
    Ok(res)
}

pub fn traverse_properties<V, R>(
    q_js_rt: &QuickJsRuntime,
    obj_ref: &JSValueRef,
    visitor: V,
) -> Result<HashMap<String, R>, EsError>
where
    V: Fn(&str, JSValueRef) -> Result<R, EsError>,
{
    let mut properties: *mut q::JSPropertyEnum = std::ptr::null_mut();
    let mut count: u32 = 0;

    let flags = (q::JS_GPN_STRING_MASK | q::JS_GPN_SYMBOL_MASK | q::JS_GPN_ENUM_ONLY) as i32;
    let ret = unsafe {
        q::JS_GetOwnPropertyNames(
            q_js_rt.context,
            &mut properties,
            &mut count,
            *obj_ref.borrow_value(),
            flags,
        )
    };
    if ret != 0 {
        return Err(EsError::new_str("Could not get object properties"));
    }

    let properties = DroppableValue::new(properties, |&mut properties| {
        for index in 0..count {
            let prop = unsafe { properties.offset(index as isize) };
            unsafe {
                q::JS_FreeAtom(q_js_rt.context, (*prop).atom);
            }
        }
        unsafe {
            q::js_free(q_js_rt.context, properties as *mut std::ffi::c_void);
        }
    });

    let mut map = HashMap::new();

    for index in 0..count {
        let prop = unsafe { (*properties).offset(index as isize) };
        let raw_value = unsafe {
            q::JS_GetPropertyInternal(
                q_js_rt.context,
                *obj_ref.borrow_value(),
                (*prop).atom,
                *obj_ref.borrow_value(),
                0,
            )
        };
        let prop_val_ref = JSValueRef::new(
            raw_value,
            false,
            true,
            "objects::traverseproperty_names raw_value",
        );
        if prop_val_ref.is_exception() {
            return Err(EsError::new_str("Could not get object property"));
        }

        let key_value = unsafe { q::JS_AtomToString(q_js_rt.context, (*prop).atom) };
        let key_ref = JSValueRef::new(
            key_value,
            false,
            true,
            "objects::traverseproperty_names key_value",
        );
        if key_ref.is_exception() {
            return Err(EsError::new_str("Could not get object property name"));
        }

        let key_str = crate::quickjs_utils::primitives::to_string(q_js_rt, &key_ref)?;
        let r = visitor(key_str.as_str(), key_ref)?;

        map.insert(key_str, r);
    }

    Ok(map)
}

pub fn is_instance_of(
    q_js_rt: &QuickJsRuntime,
    obj_ref: &JSValueRef,
    constructor_ref: JSValueRef,
) -> bool {
    if !obj_ref.is_object() {
        return false;
    }
    unsafe {
        q::JS_IsInstanceOf(
            q_js_rt.context,
            *obj_ref.borrow_value(),
            *constructor_ref.borrow_value(),
        ) > 0
    }
}

pub fn is_instance_of_by_name(
    q_js_rt: &QuickJsRuntime,
    obj_ref: &JSValueRef,
    constructor_name: &str,
) -> Result<bool, EsError> {
    if !obj_ref.is_object() {
        return Ok(false);
    }

    let constructor_ref = get_constructor(q_js_rt, constructor_name)?;
    if !constructor_ref.is_object() {
        return Ok(false);
    }

    Ok(is_instance_of(q_js_rt, obj_ref, constructor_ref))
}

#[cfg(test)]
pub mod tests {
    use crate::esruntime::EsRuntime;
    use crate::esscript::EsScript;
    use crate::quickjs_utils::objects::{
        create_object, get_property, get_property_names, set_property,
    };
    use crate::quickjs_utils::primitives::{from_i32, to_i32};
    use crate::quickjs_utils::{get_global, primitives};
    use std::sync::Arc;

    #[test]
    fn test_get_refs() {
        let rt: Arc<EsRuntime> = crate::esruntime::tests::TEST_ESRT.clone();
        rt.add_to_event_queue_sync(|q_js_rt| {
            let obj = create_object(q_js_rt).ok().expect("a");
            let prop_ref = create_object(q_js_rt).ok().expect("b");
            let prop2_ref = create_object(q_js_rt).ok().expect("c");
            assert_eq!(obj.get_ref_count(), 1);
            assert_eq!(prop_ref.get_ref_count(), 1);
            set_property(q_js_rt, &obj, "a", prop_ref.clone())
                .ok()
                .expect("d");
            assert_eq!(prop_ref.get_ref_count(), 2);
            set_property(q_js_rt, &obj, "b", prop_ref.clone())
                .ok()
                .expect("e");
            assert_eq!(prop_ref.get_ref_count(), 3);
            set_property(q_js_rt, &obj, "b", prop2_ref.clone())
                .ok()
                .expect("f");
            assert_eq!(prop_ref.get_ref_count(), 2);
            assert_eq!(prop2_ref.get_ref_count(), 2);

            let p3 = get_property(q_js_rt, &obj, "b").ok().expect("g");
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
            let obj_a = create_object(q_js_rt).ok().unwrap();
            let obj_b = create_object(q_js_rt).ok().unwrap();
            set_property(q_js_rt, &obj_a, "b", obj_b).ok().unwrap();

            let b1 = get_property(q_js_rt, &obj_a, "b").ok().unwrap();
            set_property(q_js_rt, &b1, "i", primitives::from_i32(123))
                .ok()
                .unwrap();
            drop(b1);
            q_js_rt.gc();
            let b2 = get_property(q_js_rt, &obj_a, "b").ok().unwrap();
            let i_ref = get_property(q_js_rt, &b2, "i").ok().unwrap();
            let i = to_i32(&i_ref).ok().unwrap();
            drop(i_ref);
            q_js_rt.gc();
            let i_ref2 = get_property(q_js_rt, &b2, "i").ok().unwrap();
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
            let obj_ref = q_js_rt
                .eval(EsScript::new("test_propnames.es", "({one: 1, two: 2});"))
                .ok()
                .expect("could not get test obj");
            let prop_names = get_property_names(q_js_rt, &obj_ref)
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
            let obj_ref = create_object(q_js_rt).ok().unwrap();

            assert_eq!(obj_ref.get_ref_count(), 1);

            let global_ref = get_global(q_js_rt);
            set_property(q_js_rt, &global_ref, "test_obj", obj_ref.clone())
                .ok()
                .expect("could not set property 1");

            assert_eq!(obj_ref.get_ref_count(), 2);

            let prop_ref = from_i32(123);
            let obj_ref = get_property(q_js_rt, &global_ref, "test_obj")
                .ok()
                .expect("could not get test_obj");
            set_property(q_js_rt, &obj_ref, "test_prop", prop_ref)
                .ok()
                .expect("could not set property 2");

            drop(global_ref);

            q_js_rt.gc();

            let a = q_js_rt
                .eval(EsScript::new("test_set_prop.es", "(test_obj);"))
                .ok()
                .unwrap()
                .is_object();
            assert!(a);
            let b = q_js_rt
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
