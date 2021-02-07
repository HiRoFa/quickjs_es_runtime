use crate::eserror::EsError;
use crate::quickjs_utils;
use crate::quickjscontext::QuickJsContext;
use crate::valueref::JSValueRef;
use libquickjs_sys as q;
use std::ffi::CString;

pub fn parse_q(q_ctx: &QuickJsContext, input: &str) -> Result<JSValueRef, EsError> {
    unsafe { parse(q_ctx.context, input) }
}

/// Parse a JSON string into an Object
/// please note that JSON.parse requires member names to be enclosed in double quotes
/// so {a: 1} and {'a': 1} will both fail
/// {"a": 1} will parse ok
/// # Example
/// ```dontrun
/// use quickjs_runtime::esruntimebuilder::EsRuntimeBuilder;
/// use quickjs_runtime::quickjs_utils::{json, objects, primitives};
/// use quickjs_runtime::quickjs_utils::json::parse;
/// let rt = EsRuntimeBuilder::new().build();
/// rt.add_to_event_queue_sync(|q_js_rt| {
///     let q_ctx = q_js_rt.get_main_context();
///     let parse_res = json::parse_q(q_ctx, "{\"aaa\": 165}");
///     if parse_res.is_err() {
///         panic!("could not parse: {}", parse_res.err().unwrap());
///     }
///     let obj_ref = parse_res.ok().unwrap();
///     let a_ref = objects::get_property(q_ctx.context, &obj_ref, "aaa").ok().unwrap();
///     let i = primitives::to_i32(&a_ref).ok().unwrap();
///     assert_eq!(165, i);
/// });
/// rt.gc_sync();
/// ```
/// # Safety
/// When passing a context pointer please make sure the corresponding QuickJsContext is still valid
pub unsafe fn parse(context: *mut q::JSContext, input: &str) -> Result<JSValueRef, EsError> {
    let s = CString::new(input).ok().unwrap();
    let f_n = CString::new("JSON.parse").ok().unwrap();

    #[cfg(target_pointer_width = "64")]
    let len = input.len() as u64;
    #[cfg(target_pointer_width = "32")]
    let len = input.len() as u32;

    let val = q::JS_ParseJSON(context, s.as_ptr(), len, f_n.as_ptr());

    let ret = JSValueRef::new(context, val, false, true, "json::parse result");

    if ret.is_exception() {
        if let Some(ex) = QuickJsContext::get_exception(context) {
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
/// use quickjs_runtime::esruntimebuilder::EsRuntimeBuilder;
/// use quickjs_runtime::quickjs_utils::{json, objects, primitives};
/// let rt = EsRuntimeBuilder::new().build();
/// rt.add_to_event_queue_sync(|q_js_rt| {
///     let q_ctx = q_js_rt.get_main_context();
///     let obj_ref = objects::create_object_q(q_ctx).ok().unwrap();
///     objects::set_property_q(q_ctx, &obj_ref, "a", &primitives::from_i32(741)).ok().unwrap();
///     let str_ref = json::stringify_q(q_ctx, &obj_ref, None).ok().unwrap();
///     let str_str = primitives::to_string_q(q_ctx, &str_ref).ok().unwrap();
///     assert_eq!("{\"a\":741}", str_str);
/// });
/// rt.gc_sync();
/// ```
pub fn stringify_q(
    q_ctx: &QuickJsContext,
    input: &JSValueRef,
    opt_space: Option<JSValueRef>,
) -> Result<JSValueRef, EsError> {
    unsafe { stringify(q_ctx.context, input, opt_space) }
}

/// # Safety
/// When passing a context pointer please make sure the corresponding QuickJsContext is still valid
pub unsafe fn stringify(
    context: *mut q::JSContext,
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

    let val = q::JS_JSONStringify(
        context,
        *input.borrow_value(),
        quickjs_utils::new_null(),
        *space_ref.borrow_value(),
    );
    let ret = JSValueRef::new(context, val, false, true, "json::stringify result");

    if ret.is_exception() {
        if let Some(ex) = QuickJsContext::get_exception(context) {
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
            let q_ctx = q_js_rt.get_main_context();

            let obj = objects::create_object_q(q_ctx).ok().unwrap();
            objects::set_property_q(q_ctx, &obj, "a", &primitives::from_i32(532))
                .ok()
                .unwrap();
            objects::set_property_q(q_ctx, &obj, "b", &primitives::from_bool(true))
                .ok()
                .unwrap();
            objects::set_property_q(
                q_ctx,
                &obj,
                "c",
                &primitives::from_string_q(q_ctx, "abc").ok().unwrap(),
            )
            .ok()
            .unwrap();
            let str_res = json::stringify_q(q_ctx, &obj, None).ok().unwrap();
            assert_eq!(str_res.get_ref_count(), 1);
            let json = primitives::to_string_q(q_ctx, &str_res).ok().unwrap();
            assert_eq!(json.as_str(), "{\"a\":532,\"b\":true,\"c\":\"abc\"}");

            let obj2 = json::parse_q(q_ctx, json.as_str()).ok().unwrap();

            assert_eq!(
                532,
                primitives::to_i32(&objects::get_property_q(q_ctx, &obj2, "a").ok().unwrap())
                    .ok()
                    .unwrap()
            );
        });
    }
}
