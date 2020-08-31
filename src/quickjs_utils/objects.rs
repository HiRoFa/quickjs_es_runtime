use crate::eserror::EsError;
use crate::quickjsruntime::{make_cstring, OwnedValueRef, QuickJsRuntime};

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
