use crate::eserror::EsError;
use crate::quickjs_utils;
use crate::quickjsruntime::QuickJsRuntime;
use crate::valueref::JSValueRef;
use libquickjs_sys as q;
use std::ffi::CString;

/// Parse a JSON string into an Object
/// please note that JSON.parse requires member names to be enclosed in double quotes
/// so {a: 1} and {'a': 1} will both fail
/// {"a": 1} will parse ok
/// # Example
/// ```rust
/// use quickjs_es_runtime::esruntimebuilder::EsRuntimeBuilder;
/// use quickjs_es_runtime::quickjs_utils::{json, objects, primitives};
/// let rt = EsRuntimeBuilder::new().build();
/// rt.add_to_event_queue_sync(|q_js_rt| {
///     let obj_ref = json::parse(q_js_rt, "{\"a\": 165}").ok().unwrap();
///     let a_ref = objects::get_property(q_js_rt, &obj_ref, "a").ok().unwrap();
///     let i = primitives::to_i32(&a_ref).ok().unwrap();
///     assert_eq!(165, i);
/// });
/// ```
pub fn parse(q_js_rt: &QuickJsRuntime, input: &str) -> Result<JSValueRef, EsError> {
    //pub fn JS_ParseJSON(
    //         ctx: *mut JSContext,
    //         buf: *const ::std::os::raw::c_char,
    //         buf_len: size_t,
    //         filename: *const ::std::os::raw::c_char,
    //     ) -> JSValue;

    let s = CString::new(input).ok().unwrap();
    let f_n = CString::new("JSON.parse").ok().unwrap();
    let val = unsafe {
        q::JS_ParseJSON(
            q_js_rt.context,
            s.as_ptr(),
            input.len() as u64,
            f_n.as_ptr(),
        )
    };

    let ret = JSValueRef::new(val, false, true, "json::parse result");

    if ret.is_exception() {
        if let Some(ex) = q_js_rt.get_exception() {
            Err(ex)
        } else {
            Err(EsError::new_str("unknown error while parsing json"))
        }
    } else {
        Ok(ret)
    }
}

/// Stringify an Object in script
/// # Example
/// ```rust
/// use quickjs_es_runtime::esruntimebuilder::EsRuntimeBuilder;
/// use quickjs_es_runtime::quickjs_utils::{json, objects, primitives};
/// let rt = EsRuntimeBuilder::new().build();
/// rt.add_to_event_queue_sync(|q_js_rt| {
///     let obj_ref = objects::create_object(q_js_rt).ok().unwrap();
///     objects::set_property(q_js_rt, &obj_ref, "a", primitives::from_i32(741)).ok().unwrap();
///     let str_ref = json::stringify(q_js_rt, &obj_ref, None).ok().unwrap();
///     let str_str = primitives::to_string(q_js_rt, &str_ref).ok().unwrap();
///     assert_eq!("{\"a\":741}", str_str);
/// });
/// ```
pub fn stringify(
    q_js_rt: &QuickJsRuntime,
    input: &JSValueRef,
    opt_space: Option<JSValueRef>,
) -> Result<JSValueRef, EsError> {
    //pub fn JS_JSONStringify(
    //         ctx: *mut JSContext,
    //         obj: JSValue,
    //         replacer: JSValue,
    //         space0: JSValue,
    //     ) -> JSValue;

    let space_ref = match opt_space {
        None => quickjs_utils::new_null_ref(),
        Some(s) => s,
    };

    let val = unsafe {
        q::JS_JSONStringify(
            q_js_rt.context,
            *input.borrow_value(),
            quickjs_utils::new_null(),
            *space_ref.borrow_value(),
        )
    };
    let ret = JSValueRef::new(val, false, true, "json::stringify result");

    if ret.is_exception() {
        if let Some(ex) = q_js_rt.get_exception() {
            Err(ex)
        } else {
            Err(EsError::new_str("unknown error in json::stringify"))
        }
    } else {
        Ok(ret)
    }
}

#[cfg(test)]
pub mod tests {
    use crate::esruntime::EsRuntime;
    use crate::quickjs_utils::{json, objects, primitives};
    use std::sync::Arc;

    #[test]
    fn test_json() {
        let rt: Arc<EsRuntime> = crate::esruntime::tests::TEST_ESRT.clone();
        rt.add_to_event_queue_sync(|q_js_rt| {
            let obj = objects::create_object(q_js_rt).ok().unwrap();
            objects::set_property(q_js_rt, &obj, "a", primitives::from_i32(532))
                .ok()
                .unwrap();
            objects::set_property(q_js_rt, &obj, "b", primitives::from_bool(true))
                .ok()
                .unwrap();
            objects::set_property(
                q_js_rt,
                &obj,
                "c",
                primitives::from_string(q_js_rt, "abc").ok().unwrap(),
            )
            .ok()
            .unwrap();
            let str_res = json::stringify(q_js_rt, &obj, None).ok().unwrap();
            assert_eq!(str_res.get_ref_count(), 1);
            let json = primitives::to_string(q_js_rt, &str_res).ok().unwrap();
            assert_eq!(json.as_str(), "{\"a\":532,\"b\":true,\"c\":\"abc\"}");

            let obj2 = json::parse(q_js_rt, json.as_str()).ok().unwrap();

            assert_eq!(
                532,
                primitives::to_i32(&objects::get_property(q_js_rt, &obj2, "a").ok().unwrap())
                    .ok()
                    .unwrap()
            );
        });
    }
}
