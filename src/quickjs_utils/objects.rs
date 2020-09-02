use crate::droppable_value::DroppableValue;
use crate::eserror::EsError;
use crate::quickjs_utils::get_global;
use crate::quickjsruntime::{make_cstring, OwnedValueRef, QuickJsRuntime};
use libquickjs_sys as q;
use std::collections::HashMap;

#[allow(dead_code)]
pub fn construct_object(
    _q_js_rt: &QuickJsRuntime,
    _constructor_ref: &OwnedValueRef,
) -> Result<OwnedValueRef, EsError> {
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

pub fn create_object(q_js_rt: &QuickJsRuntime) -> Result<OwnedValueRef, EsError> {
    let obj = unsafe { q::JS_NewObject(q_js_rt.context) };
    let obj_ref = OwnedValueRef::new(obj);
    if obj_ref.is_exception() {
        return Err(EsError::new_str("Could not create object"));
    }
    Ok(obj_ref)
}

pub fn set_property(
    q_js_rt: &QuickJsRuntime,
    obj_ref: &OwnedValueRef,
    prop_name: &str,
    prop_ref: OwnedValueRef,
) -> Result<(), EsError> {
    let ckey = make_cstring(prop_name.clone());

    let mut prop_ref = prop_ref;

    let ret = unsafe {
        q::JS_DefinePropertyValueStr(
            q_js_rt.context,
            *obj_ref.borrow_value(),
            ckey.ok().unwrap().as_ptr(),
            prop_ref.consume_value(),
            q::JS_PROP_C_W_E as i32,
        )
    };
    if ret < 0 {
        return Err(EsError::new_str("Could not add property to object"));
    }
    Ok(())
}

pub fn get_property(
    q_js_rt: &QuickJsRuntime,
    obj_ref: &OwnedValueRef,
    prop_name: &str,
) -> Result<OwnedValueRef, EsError> {
    if obj_ref.is_null() || obj_ref.is_undefined() {
        return Err(EsError::new_str(
            "could not get prop from null or undefined",
        ));
    }

    let c_prop_name = make_cstring(prop_name)
        .ok()
        .expect("could not create cstring");

    let prop_val = unsafe {
        q::JS_GetPropertyStr(
            q_js_rt.context,
            *obj_ref.borrow_value(),
            c_prop_name.as_ptr(),
        )
    };
    Ok(OwnedValueRef::new(prop_val))
}

#[allow(dead_code)]
pub fn get_property_names(
    q_js_rt: &QuickJsRuntime,
    obj_ref: &OwnedValueRef,
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
        let key_ref = OwnedValueRef::new(key_value);
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
    obj_ref: &OwnedValueRef,
    visitor: V,
) -> Result<HashMap<String, R>, EsError>
where
    V: Fn(&str, OwnedValueRef) -> Result<R, EsError>,
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
        let prop_val_ref = OwnedValueRef::new(raw_value);
        if prop_val_ref.is_exception() {
            return Err(EsError::new_str("Could not get object property"));
        }

        let key_value = unsafe { q::JS_AtomToString(q_js_rt.context, (*prop).atom) };
        let key_ref = OwnedValueRef::new(key_value);
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
    obj_ref: &OwnedValueRef,
    constructor_ref: OwnedValueRef,
) -> bool {
    if !obj_ref.is_object() {
        return false;
    }
    let ret = unsafe {
        q::JS_IsInstanceOf(
            q_js_rt.context,
            *obj_ref.borrow_value(),
            *constructor_ref.borrow_value(),
        ) > 0
    };

    ret
}

pub fn is_instance_of_by_name(
    q_js_rt: &QuickJsRuntime,
    obj_ref: &OwnedValueRef,
    constructor_name: &str,
) -> Result<bool, EsError> {
    if !obj_ref.is_object() {
        return Ok(false);
    }

    let global_ref = get_global(q_js_rt);

    let constructor_ref = get_property(q_js_rt, &global_ref, constructor_name)?;
    if !constructor_ref.is_object() {
        return Ok(false);
    }

    Ok(is_instance_of(q_js_rt, obj_ref, constructor_ref))
}

#[cfg(test)]
pub mod tests {
    use crate::esruntime::EsRuntime;
    use crate::esscript::EsScript;
    use crate::quickjs_utils::get_global;
    use crate::quickjs_utils::objects::{
        create_object, get_property, get_property_names, set_property,
    };
    use crate::quickjs_utils::primitives::from_i32;
    use std::sync::Arc;

    #[test]
    fn test_propnames() {
        let rt: Arc<EsRuntime> = crate::esruntime::tests::TEST_ESRT.clone();
        let io = rt.add_to_event_queue_sync(|q_js_rt| {
            let obj_ref = q_js_rt
                .eval(EsScript::new(
                    "test_propnames.es".to_string(),
                    "({one: 1, two: 2});".to_string(),
                ))
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
        assert!(io)
    }

    #[test]
    fn test_set_prop() {
        let rt: Arc<EsRuntime> = crate::esruntime::tests::TEST_ESRT.clone();
        let io = rt.add_to_event_queue_sync(|q_js_rt| {
            let obj_ref = create_object(q_js_rt).ok().unwrap();
            let global_ref = get_global(q_js_rt);
            set_property(q_js_rt, &global_ref, "test_obj", obj_ref)
                .ok()
                .expect("could not set property 1");
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
                .eval(EsScript::new(
                    "test_set_prop.es".to_string(),
                    "(test_obj);".to_string(),
                ))
                .ok()
                .unwrap()
                .is_object();
            assert!(a);
            let b = q_js_rt
                .eval(EsScript::new(
                    "test_set_prop.es".to_string(),
                    "(test_obj.test_prop);".to_string(),
                ))
                .ok()
                .unwrap()
                .is_i32();
            assert!(b);
            a && b
        });
        assert!(io);
    }
}
