use crate::eserror::EsError;
use crate::quickjsruntime::{OwnedValueRef, QuickJsRuntime};
use libquickjs_sys as q;

pub fn is_promise(q_js_rt: &QuickJsRuntime, obj_ref: &OwnedValueRef) -> Result<bool, EsError> {
    if !obj_ref.is_object() {
        Ok(false)
    } else {
        let promise_constructor = get_promise_constructor(q_js_rt)?;
        // todo move to is_instanceof util
        let is_prom = unsafe {
            q::JS_IsInstanceOf(q_js_rt.context, obj_ref.value, promise_constructor.value) > 0
        };
        Ok(is_prom)
    }
}

pub fn get_promise_constructor(q_js_rt: &QuickJsRuntime) -> Result<OwnedValueRef, EsError> {
    // todo move to get_global
    let global = unsafe { q::JS_GetGlobalObject(q_js_rt.context) };
    let global_ref = OwnedValueRef::new(global);

    // tdo use objects::get_prop, currently cstr is not freed
    let promise_constructor = unsafe {
        q::JS_GetPropertyStr(
            q_js_rt.context,
            global_ref.value,
            std::ffi::CStr::from_bytes_with_nul(b"Promise\0")
                .unwrap()
                .as_ptr(),
        )
    };
    let promise_constructor_ref = OwnedValueRef::new(promise_constructor);
    if !promise_constructor_ref.is_object() {
        return Err(EsError::new_str("could not get Promise"));
    }

    Ok(promise_constructor_ref)
}
