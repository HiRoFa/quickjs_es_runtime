use crate::jsutils::JsError;
use crate::quickjs_utils;
use crate::quickjs_utils::{functions, primitives};
use crate::quickjsrealmadapter::QuickJsRealmAdapter;
use crate::quickjsvalueadapter::QuickJsValueAdapter;
#[cfg(feature = "bellard")]
use crate::quickjsvalueadapter::TAG_BIG_INT;
use libquickjs_sys as q;

pub fn new_bigint_i64_q(
    context: &QuickJsRealmAdapter,
    int: i64,
) -> Result<QuickJsValueAdapter, JsError> {
    unsafe { new_bigint_i64(context.context, int) }
}

pub fn new_bigint_u64_q(
    context: &QuickJsRealmAdapter,
    int: u64,
) -> Result<QuickJsValueAdapter, JsError> {
    unsafe { new_bigint_u64(context.context, int) }
}

#[allow(dead_code)]
/// # Safety
/// When passing a context pointer please make sure the corresponding QuickJsContext is still valid
pub unsafe fn new_bigint_i64(
    context: *mut q::JSContext,
    int: i64,
) -> Result<QuickJsValueAdapter, JsError> {
    let res_val = q::JS_NewBigInt64(context, int);
    let ret = QuickJsValueAdapter::new(context, res_val, false, true, "new_bigint_i64");

    #[cfg(feature = "bellard")]
    {
        #[cfg(debug_assertions)]
        if ret.get_tag() == TAG_BIG_INT {
            assert_eq!(ret.get_ref_count(), 1);
        }
    }
    Ok(ret)
}

#[allow(dead_code)]
/// # Safety
/// When passing a context pointer please make sure the corresponding QuickJsContext is still valid
pub unsafe fn new_bigint_u64(
    context: *mut q::JSContext,
    int: u64,
) -> Result<QuickJsValueAdapter, JsError> {
    let res_val = q::JS_NewBigUint64(context, int);
    let ret = QuickJsValueAdapter::new(context, res_val, false, true, "new_bigint_u64");

    #[cfg(feature = "bellard")]
    {
        #[cfg(debug_assertions)]
        if ret.get_tag() == TAG_BIG_INT {
            assert_eq!(ret.get_ref_count(), 1);
        }
    }
    Ok(ret)
}

pub fn new_bigint_str_q(
    context: &QuickJsRealmAdapter,
    input_str: &str,
) -> Result<QuickJsValueAdapter, JsError> {
    unsafe { new_bigint_str(context.context, input_str) }
}

#[allow(dead_code)]
/// # Safety
/// When passing a context pointer please make sure the corresponding QuickJsContext is still valid
pub unsafe fn new_bigint_str(
    context: *mut q::JSContext,
    input_str: &str,
) -> Result<QuickJsValueAdapter, JsError> {
    let global_ref = quickjs_utils::get_global(context);
    let str_ref = primitives::from_string(context, input_str)?;
    let bigint_ref = functions::invoke_member_function(context, &global_ref, "BigInt", &[str_ref])?;
    let ret = bigint_ref;

    #[cfg(feature = "bellard")]
    assert_eq!(ret.get_ref_count(), 1);
    Ok(ret)
}

pub fn to_string_q(
    context: &QuickJsRealmAdapter,
    big_int_ref: &QuickJsValueAdapter,
) -> Result<String, JsError> {
    unsafe { to_string(context.context, big_int_ref) }
}

#[allow(dead_code)]
/// # Safety
/// When passing a context pointer please make sure the corresponding QuickJsContext is still valid
pub unsafe fn to_string(
    context: *mut q::JSContext,
    big_int_ref: &QuickJsValueAdapter,
) -> Result<String, JsError> {
    if !big_int_ref.is_big_int() {
        return Err(JsError::new_str("big_int_ref was not a big_int"));
    }
    functions::call_to_string(context, big_int_ref)
}

#[cfg(test)]
pub mod tests {
    use crate::facades::tests::init_test_rt;
    use crate::jsutils::Script;
    use crate::quickjs_utils::bigints;
    use crate::quickjs_utils::bigints::new_bigint_str_q;

    #[test]
    fn test_bigint() {
        let rt = init_test_rt();
        rt.exe_rt_task_in_event_loop(|q_js_rt| {
            let q_ctx = q_js_rt.get_main_realm();

            let res = q_ctx
                .eval(Script::new("createABigInt.js", "BigInt(1234567890)"))
                .expect("script failed");
            log::info!(
                "script bi was {} {}",
                res.get_tag(),
                res.to_string().expect("could not toString")
            );

            let bi_ref = bigints::new_bigint_u64_q(q_ctx, 659863456456)
                .expect("could not create bigint from u64");

            unsafe {
                if let Some(e) = crate::quickjs_utils::errors::get_exception(q_ctx.context) {
                    log::error!("ex: {}", e);
                }
            }

            let to_str = bigints::to_string_q(q_ctx, &bi_ref).expect("could not tostring bigint");
            assert_eq!(to_str, "659863456456");
            let bi_ref = bigints::new_bigint_i64_q(q_ctx, 659863456457)
                .expect("could not create bigint from u64");
            let to_str = bigints::to_string_q(q_ctx, &bi_ref).expect("could not tostring bigint");
            assert_eq!(to_str, "659863456457");

            let bi_ref =
                new_bigint_str_q(q_ctx, "345346345645234564536345345345345456534783448567")
                    .expect("could not create bigint from str");

            log::debug!("bi_ref.get_js_type is {}", bi_ref.get_js_type());

            let to_str = bigints::to_string_q(q_ctx, &bi_ref).expect("could not tostring bigint");
            assert_eq!(to_str, "345346345645234564536345345345345456534783448567");
        });
    }
}
