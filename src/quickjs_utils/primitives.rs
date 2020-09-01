use crate::eserror::EsError;
use crate::quickjsruntime::{
    OwnedValueRef, QuickJsRuntime, TAG_BOOL, TAG_FLOAT64, TAG_INT, TAG_STRING,
};
use libquickjs_sys as q;
use libquickjs_sys::JSValue as JSVal;
use std::os::raw::c_char;

pub fn is_bool(value_ref: &OwnedValueRef) -> bool {
    value_ref.value.tag == TAG_BOOL
}

pub fn to_bool(value_ref: &OwnedValueRef) -> Result<bool, EsError> {
    let r = &value_ref.value;
    if r.tag == TAG_BOOL {
        let raw = unsafe { r.u.int32 };
        let val: bool = raw > 0;
        Ok(val)
    } else {
        Err(EsError::new_str("value is not a boolean"))
    }
}

pub fn from_bool(b: bool) -> OwnedValueRef {
    OwnedValueRef::new(q::JSValue {
        u: q::JSValueUnion {
            int32: if b { 1 } else { 0 },
        },
        tag: TAG_BOOL,
    })
}

pub fn is_f64(value_ref: &OwnedValueRef) -> bool {
    value_ref.value.tag == TAG_FLOAT64
}

pub fn to_f64(value_ref: &OwnedValueRef) -> Result<f64, EsError> {
    let r = &value_ref.value;
    if r.tag == TAG_FLOAT64 {
        let val = unsafe { r.u.float64 };
        Ok(val)
    } else {
        Err(EsError::new_str("value was not a float64"))
    }
}

pub fn from_f64(f: f64) -> OwnedValueRef {
    OwnedValueRef::new(q::JSValue {
        u: q::JSValueUnion { float64: f },
        tag: TAG_FLOAT64,
    })
}

pub fn is_i32(value_ref: &OwnedValueRef) -> bool {
    value_ref.value.tag == TAG_INT
}

pub fn to_i32(value_ref: &OwnedValueRef) -> Result<i32, EsError> {
    let r = &value_ref.value;

    if r.tag == TAG_INT {
        let val: i32 = unsafe { r.u.int32 };
        Ok(val)
    } else {
        Err(EsError::new_str("val is not an int"))
    }
}

pub fn from_i32(i: i32) -> OwnedValueRef {
    OwnedValueRef::new(JSVal {
        u: q::JSValueUnion { int32: i },
        tag: TAG_INT,
    })
}

pub fn is_string(value_ref: &OwnedValueRef) -> bool {
    value_ref.value.tag == TAG_STRING
}

pub fn to_string(q_js_rt: &QuickJsRuntime, value_ref: &OwnedValueRef) -> Result<String, EsError> {
    let r = &value_ref.value;

    let ptr = unsafe { q::JS_ToCStringLen2(q_js_rt.context, std::ptr::null_mut(), *r, 0) };

    if ptr.is_null() {
        return Err(EsError::new_str(
            "Could not convert string: got a null pointer",
        ));
    }

    let cstr = unsafe { std::ffi::CStr::from_ptr(ptr) };

    let s = cstr.to_str().ok().expect("invalid string").to_string();

    // Free the c string.
    unsafe { q::JS_FreeCString(q_js_rt.context, ptr) };

    Ok(s)
}

pub fn from_string(q_js_rt: &QuickJsRuntime, s: &str) -> Result<OwnedValueRef, EsError> {
    let qval =
        unsafe { q::JS_NewStringLen(q_js_rt.context, s.as_ptr() as *const c_char, s.len() as _) };
    let ret = OwnedValueRef::new(qval);
    if ret.is_exception() {
        return Err(EsError::new_str("Could not create string in runtime"));
    }

    Ok(ret)
}
