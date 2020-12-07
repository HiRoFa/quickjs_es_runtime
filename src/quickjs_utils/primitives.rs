use crate::eserror::EsError;
use crate::valueref::{JSValueRef, TAG_BOOL, TAG_FLOAT64, TAG_INT};
use libquickjs_sys as q;
use libquickjs_sys::JSValue as JSVal;
use std::os::raw::c_char;

pub fn to_bool(value_ref: &JSValueRef) -> Result<bool, EsError> {
    if value_ref.is_bool() {
        let r = value_ref.borrow_value();
        let raw = unsafe { r.u.int32 };
        let val: bool = raw > 0;
        Ok(val)
    } else {
        Err(EsError::new_str("value is not a boolean"))
    }
}

pub fn from_bool(b: bool) -> JSValueRef {
    JSValueRef::new_no_context(
        q::JSValue {
            u: q::JSValueUnion {
                int32: if b { 1 } else { 0 },
            },
            tag: TAG_BOOL,
        },
        "primitives::from_bool",
    )
}

pub fn to_f64(value_ref: &JSValueRef) -> Result<f64, EsError> {
    if value_ref.is_f64() {
        let r = value_ref.borrow_value();
        let val = unsafe { r.u.float64 };
        Ok(val)
    } else {
        Err(EsError::new_str("value was not a float64"))
    }
}

pub fn from_f64(f: f64) -> JSValueRef {
    JSValueRef::new_no_context(
        q::JSValue {
            u: q::JSValueUnion { float64: f },
            tag: TAG_FLOAT64,
        },
        "primitives::from_f64",
    )
}

pub fn to_i32(value_ref: &JSValueRef) -> Result<i32, EsError> {
    if value_ref.is_i32() {
        let r = value_ref.borrow_value();
        let val: i32 = unsafe { r.u.int32 };
        Ok(val)
    } else {
        Err(EsError::new_str("val is not an int"))
    }
}

pub fn from_i32(i: i32) -> JSValueRef {
    JSValueRef::new_no_context(
        JSVal {
            u: q::JSValueUnion { int32: i },
            tag: TAG_INT,
        },
        "primitives::from_i32",
    )
}

pub fn to_string(context: *mut q::JSContext, value_ref: &JSValueRef) -> Result<String, EsError> {
    log::trace!("primitives::to_string on {}", value_ref.borrow_value().tag);

    assert!(value_ref.is_string());

    let ptr =
        unsafe { q::JS_ToCStringLen2(context, std::ptr::null_mut(), *value_ref.borrow_value(), 0) };

    if ptr.is_null() {
        return Err(EsError::new_str(
            "Could not convert string: got a null pointer",
        ));
    }

    let cstr = unsafe { std::ffi::CStr::from_ptr(ptr) };

    let s = cstr.to_str().expect("invalid string").to_string();

    // Free the c string.
    unsafe { q::JS_FreeCString(context, ptr) };

    Ok(s)
}

pub fn to_str(context: *mut q::JSContext, value_ref: &JSValueRef) -> Result<&str, EsError> {
    log::trace!("primitives::to_string on {}", value_ref.borrow_value().tag);

    assert!(value_ref.is_string());

    let ptr =
        unsafe { q::JS_ToCStringLen2(context, std::ptr::null_mut(), *value_ref.borrow_value(), 0) };

    if ptr.is_null() {
        return Err(EsError::new_str(
            "Could not convert string: got a null pointer",
        ));
    }

    let cstr = unsafe { std::ffi::CStr::from_ptr(ptr) };

    let s = cstr.to_str().expect("invalid string");

    // Free the c string.
    unsafe { q::JS_FreeCString(context, ptr) };

    Ok(s)
}

pub fn from_string(context: *mut q::JSContext, s: &str) -> Result<JSValueRef, EsError> {
    let qval = unsafe { q::JS_NewStringLen(context, s.as_ptr() as *const c_char, s.len() as _) };
    let ret = JSValueRef::new(context, qval, false, true, "primitives::from_string qval");
    if ret.is_exception() {
        return Err(EsError::new_str("Could not create string in runtime"));
    }

    Ok(ret)
}
