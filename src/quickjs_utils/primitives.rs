use crate::quickjscontext::QuickJsRealmAdapter;
use crate::valueref::JSValueRef;
use core::ptr;
use hirofa_utils::js_utils::JsError;
use libquickjs_sys as q;
use std::os::raw::c_char;

pub fn to_bool(value_ref: &JSValueRef) -> Result<bool, JsError> {
    if value_ref.is_bool() {
        let r = value_ref.borrow_value();
        let raw = unsafe { r.u.int32 };
        let val: bool = raw > 0;
        Ok(val)
    } else {
        Err(JsError::new_str("value is not a boolean"))
    }
}

pub fn from_bool(b: bool) -> JSValueRef {
    let raw = unsafe { q::JS_NewBool(ptr::null_mut(), b) };
    JSValueRef::new_no_context(raw, "primitives::from_bool")
}

pub fn to_f64(value_ref: &JSValueRef) -> Result<f64, JsError> {
    if value_ref.is_f64() {
        let r = value_ref.borrow_value();
        let val = unsafe { r.u.float64 };
        Ok(val)
    } else {
        Err(JsError::new_str("value was not a float64"))
    }
}

pub fn from_f64(f: f64) -> JSValueRef {
    let raw = unsafe { q::JS_NewFloat64(ptr::null_mut(), f) };
    JSValueRef::new_no_context(raw, "primitives::from_f64")
}

pub fn to_i32(value_ref: &JSValueRef) -> Result<i32, JsError> {
    if value_ref.is_i32() {
        let r = value_ref.borrow_value();
        let val: i32 = unsafe { r.u.int32 };
        Ok(val)
    } else {
        Err(JsError::new_str("val is not an int"))
    }
}

pub fn from_i32(i: i32) -> JSValueRef {
    let raw = unsafe { q::JS_NewInt32(ptr::null_mut(), i) };
    JSValueRef::new_no_context(raw, "primitives::from_i32")
}

pub fn to_string_q(q_ctx: &QuickJsRealmAdapter, value_ref: &JSValueRef) -> Result<String, JsError> {
    unsafe { to_string(q_ctx.context, value_ref) }
}
/// # Safety
/// When passing a context pointer please make sure the corresponding QuickJsContext is still valid
pub unsafe fn to_string(
    context: *mut q::JSContext,
    value_ref: &JSValueRef,
) -> Result<String, JsError> {
    log::trace!("primitives::to_string on {}", value_ref.borrow_value().tag);

    assert!(value_ref.is_string());

    let mut len = 0;

    let ptr: *const c_char = q::JS_ToCStringLen2(context, &mut len, *value_ref.borrow_value(), 0);

    if len == 0 {
        return Ok("".to_string());
    }

    if ptr.is_null() {
        return Err(JsError::new_str(
            "Could not convert string: got a null pointer",
        ));
    }

    let cstr = std::ffi::CStr::from_ptr(ptr);

    let s = cstr.to_string_lossy().into_owned();

    // Free the c string.
    q::JS_FreeCString(context, ptr);

    Ok(s)
}

pub fn from_string_q(q_ctx: &QuickJsRealmAdapter, s: &str) -> Result<JSValueRef, JsError> {
    unsafe { from_string(q_ctx.context, s) }
}
/// # Safety
/// When passing a context pointer please make sure the corresponding QuickJsContext is still valid
pub unsafe fn from_string(context: *mut q::JSContext, s: &str) -> Result<JSValueRef, JsError> {
    let qval = q::JS_NewStringLen(context, s.as_ptr() as *const c_char, s.len() as _);
    let ret = JSValueRef::new(context, qval, false, true, "primitives::from_string qval");
    if ret.is_exception() {
        return Err(JsError::new_str("Could not create string in runtime"));
    }

    Ok(ret)
}
