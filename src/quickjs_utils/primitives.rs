use crate::jsutils::JsError;
use crate::quickjsrealmadapter::QuickJsRealmAdapter;
use crate::quickjsvalueadapter::QuickJsValueAdapter;
use core::ptr;
use libquickjs_sys as q;
use std::os::raw::c_char;

pub fn to_bool(value_ref: &QuickJsValueAdapter) -> Result<bool, JsError> {
    if value_ref.is_bool() {
        let r = value_ref.borrow_value();
        #[cfg(feature = "bellard")]
        let raw = unsafe { r.u.uint64 };
        #[cfg(feature = "quickjs-ng")]
        let raw = unsafe { r.u.int32 };
        let val: bool = raw > 0;
        Ok(val)
    } else {
        Err(JsError::new_str("value is not a boolean"))
    }
}

pub fn from_bool(b: bool) -> QuickJsValueAdapter {
    let raw = unsafe { q::JS_NewBool(ptr::null_mut(), b) };
    QuickJsValueAdapter::new_no_context(raw, "primitives::from_bool")
}

pub fn to_f64(value_ref: &QuickJsValueAdapter) -> Result<f64, JsError> {
    if value_ref.is_f64() {
        let r = value_ref.borrow_value();
        let val = unsafe { r.u.float64 };
        Ok(val)
    } else {
        Err(JsError::new_str("value was not a float64"))
    }
}

pub fn from_f64(f: f64) -> QuickJsValueAdapter {
    #[cfg(feature = "bellard")]
    let raw = unsafe { q::JS_NewFloat64(ptr::null_mut(), f) };

    #[cfg(feature = "quickjs-ng")]
    let raw = unsafe { q::JS_NewNumber(ptr::null_mut(), f) };

    QuickJsValueAdapter::new_no_context(raw, "primitives::from_f64")
}

pub fn to_i32(value_ref: &QuickJsValueAdapter) -> Result<i32, JsError> {
    if value_ref.is_i32() {
        let _r = value_ref.borrow_value();
        let _val: i32 = 0;
        #[cfg(feature = "bellard")]
        let val = unsafe { q::JS_ValueGetInt(*value_ref.borrow_value()) };
        #[cfg(feature = "quickjs-ng")]
        {
            val = unsafe { r.u.int32 };
        }
        Ok(val)
    } else {
        Err(JsError::new_str("val is not an int"))
    }
}

pub fn from_i32(i: i32) -> QuickJsValueAdapter {
    let raw = unsafe { q::JS_NewInt32(ptr::null_mut(), i) };
    QuickJsValueAdapter::new_no_context(raw, "primitives::from_i32")
}

pub fn to_string_q(
    q_ctx: &QuickJsRealmAdapter,
    value_ref: &QuickJsValueAdapter,
) -> Result<String, JsError> {
    unsafe { to_string(q_ctx.context, value_ref) }
}
/// # Safety
/// When passing a context pointer please make sure the corresponding QuickJsContext is still valid
pub unsafe fn to_string(
    context: *mut q::JSContext,
    value_ref: &QuickJsValueAdapter,
) -> Result<String, JsError> {
    //log::trace!("primitives::to_string on {}", value_ref.borrow_value().tag);

    assert!(value_ref.is_string());

    let mut len = 0;

    #[cfg(feature = "bellard")]
    let ptr: *const c_char = q::JS_ToCStringLen2(context, &mut len, *value_ref.borrow_value(), 0);
    #[cfg(feature = "quickjs-ng")]
    let ptr: *const c_char =
        q::JS_ToCStringLen2(context, &mut len, *value_ref.borrow_value(), false);

    if len == 0 {
        return Ok("".to_string());
    }

    if ptr.is_null() {
        return Err(JsError::new_str(
            "Could not convert string: got a null pointer",
        ));
    }

    let bytes = std::slice::from_raw_parts(ptr as *const u8, len);

    // Convert to String (validate UTF-8).
    let s = String::from_utf8_lossy(bytes).into_owned();

    // Free the c string.
    q::JS_FreeCString(context, ptr);

    Ok(s)
}

pub fn from_string_q(q_ctx: &QuickJsRealmAdapter, s: &str) -> Result<QuickJsValueAdapter, JsError> {
    unsafe { from_string(q_ctx.context, s) }
}
/// # Safety
/// When passing a context pointer please make sure the corresponding QuickJsContext is still valid
pub unsafe fn from_string(
    context: *mut q::JSContext,
    s: &str,
) -> Result<QuickJsValueAdapter, JsError> {
    let qval = q::JS_NewStringLen(context, s.as_ptr() as *const c_char, s.len() as _);
    let ret = QuickJsValueAdapter::new(context, qval, false, true, "primitives::from_string qval");
    if ret.is_exception() {
        return Err(JsError::new_str("Could not create string in runtime"));
    }

    Ok(ret)
}

#[cfg(test)]
pub mod tests {
    use crate::facades::tests::init_test_rt;
    use crate::jsutils::Script;

    #[tokio::test]
    async fn test_emoji() {
        let rt = init_test_rt();

        let res = rt.eval(None, Script::new("testEmoji.js", "'hi'")).await;

        match res {
            Ok(fac) => {
                assert_eq!(fac.get_str(), "hi");
            }
            Err(e) => {
                panic!("script failed: {}", e);
            }
        }

        let res = rt.eval(None, Script::new("testEmoji.js", "'👍'")).await;

        match res {
            Ok(fac) => {
                assert_eq!(fac.get_str(), "👍");
            }
            Err(e) => {
                panic!("script failed: {}", e);
            }
        }

        let res = rt.eval(None, Script::new("testEmoji.js", "'pre👍'")).await;

        match res {
            Ok(fac) => {
                assert_eq!(fac.get_str(), "pre👍");
            }
            Err(e) => {
                panic!("script failed: {}", e);
            }
        }

        let res = rt.eval(None, Script::new("testEmoji.js", "'👍post'")).await;

        match res {
            Ok(fac) => {
                assert_eq!(fac.get_str(), "👍post");
            }
            Err(e) => {
                panic!("script failed: {}", e);
            }
        }

        let res = rt
            .eval(None, Script::new("testEmoji.js", "'pre👍post'"))
            .await;

        match res {
            Ok(fac) => {
                assert_eq!(fac.get_str(), "pre👍post");
            }
            Err(e) => {
                panic!("script failed: {}", e);
            }
        }

        let res = rt
            .eval(
                None,
                Script::new("testEmoji.js", "JSON.stringify({c: '👍'})"),
            )
            .await;

        match res {
            Ok(fac) => {
                assert_eq!(fac.get_str(), "{\"c\":\"👍\"}");
            }
            Err(e) => {
                panic!("script failed: {}", e);
            }
        }
    }
}
