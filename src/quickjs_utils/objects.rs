use crate::droppable_value::DroppableValue;
use crate::eserror::EsError;
use crate::quickjsruntime::{make_cstring, OwnedValueRef, QuickJsRuntime};
use libquickjs_sys as q;

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
    prop_ref: &OwnedValueRef,
) -> Result<(), EsError> {
    let ckey = make_cstring(prop_name.clone());

    let ret = unsafe {
        q::JS_DefinePropertyValueStr(
            q_js_rt.context,
            obj_ref.value,
            ckey.ok().unwrap().as_ptr(),
            prop_ref.value,
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

    let prop_val =
        unsafe { q::JS_GetPropertyStr(q_js_rt.context, obj_ref.value, c_prop_name.as_ptr()) };
    Ok(OwnedValueRef::new(prop_val))
}

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
            *&obj_ref.value,
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

pub fn traverse_properties<V>(
    q_js_rt: &QuickJsRuntime,
    obj_ref: &OwnedValueRef,
    visitor: V,
) -> Result<(), EsError>
where
    V: Fn(&str, OwnedValueRef) -> Result<(), EsError>,
{
    let mut properties: *mut q::JSPropertyEnum = std::ptr::null_mut();
    let mut count: u32 = 0;

    let flags = (q::JS_GPN_STRING_MASK | q::JS_GPN_SYMBOL_MASK | q::JS_GPN_ENUM_ONLY) as i32;
    let ret = unsafe {
        q::JS_GetOwnPropertyNames(
            q_js_rt.context,
            &mut properties,
            &mut count,
            *&obj_ref.value,
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

    for index in 0..count {
        let prop = unsafe { (*properties).offset(index as isize) };
        let raw_value = unsafe {
            q::JS_GetPropertyInternal(
                q_js_rt.context,
                *&obj_ref.value,
                (*prop).atom,
                *&obj_ref.value,
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
        visitor(key_str.as_str(), key_ref)?;
    }

    Ok(())
}
