use crate::eserror::EsError;
use crate::quickjs_utils;
use crate::quickjs_utils::{functions, primitives};
use crate::quickjscontext::QuickJsContext;
use crate::valueref::JSValueRef;
use libquickjs_sys as q;

pub fn new_bigint_i64_q(context: &QuickJsContext, int: i64) -> Result<JSValueRef, EsError> {
    unsafe { new_bigint_i64(context.context, int) }
}

#[allow(dead_code)]
/// # Safety
/// When passing a context pointer please make sure the corresponding QuickJsContext is still valid
pub unsafe fn new_bigint_i64(context: *mut q::JSContext, int: i64) -> Result<JSValueRef, EsError> {
    let res_val = q::JS_NewBigInt64(context, int);
    let ret = JSValueRef::new(context, res_val, false, true, "new_bigint_i64");
    assert_eq!(ret.get_ref_count(), 1);
    Ok(ret)
}

#[allow(dead_code)]
/// # Safety
/// When passing a context pointer please make sure the corresponding QuickJsContext is still valid
pub unsafe fn new_bigint_u64(context: *mut q::JSContext, int: u64) -> Result<JSValueRef, EsError> {
    let res_val = q::JS_NewBigUint64(context, int);
    let ret = JSValueRef::new(context, res_val, false, true, "new_bigint_u64");
    assert_eq!(ret.get_ref_count(), 1);
    Ok(ret)
}

pub fn new_bigint_str_q(context: &QuickJsContext, input_str: &str) -> Result<JSValueRef, EsError> {
    unsafe { new_bigint_str(context.context, input_str) }
}

#[allow(dead_code)]
/// # Safety
/// When passing a context pointer please make sure the corresponding QuickJsContext is still valid
pub unsafe fn new_bigint_str(
    context: *mut q::JSContext,
    input_str: &str,
) -> Result<JSValueRef, EsError> {
    let global_ref = quickjs_utils::get_global(context);
    let str_ref = primitives::from_string(context, input_str)?;
    let bigint_ref =
        functions::invoke_member_function(context, &global_ref, "BigInt", vec![str_ref])?;
    let ret = bigint_ref;
    assert_eq!(ret.get_ref_count(), 1);
    Ok(ret)
}

pub fn to_string_q(context: &QuickJsContext, big_int_ref: &JSValueRef) -> Result<String, EsError> {
    unsafe { to_string(context.context, big_int_ref) }
}

#[allow(dead_code)]
/// # Safety
/// When passing a context pointer please make sure the corresponding QuickJsContext is still valid
pub unsafe fn to_string(
    context: *mut q::JSContext,
    big_int_ref: &JSValueRef,
) -> Result<String, EsError> {
    if !big_int_ref.is_big_int() {
        return Err(EsError::new_str("big_int_ref was not a big_int"));
    }
    functions::call_to_string(context, big_int_ref)
}

#[cfg(test)]
pub mod tests {
    use crate::esruntime::tests::init_test_rt;
    use crate::esruntime::EsRuntime;
    use crate::quickjs_utils::bigints;
    use crate::quickjs_utils::bigints::new_bigint_str_q;
    use std::sync::Arc;

    #[test]
    fn test_bigint() {
        let rt: Arc<EsRuntime> = init_test_rt();
        rt.exe_rt_task_in_event_loop(|q_js_rt| {
            let q_ctx = q_js_rt.get_main_context();
            let bi_ref =
                new_bigint_str_q(q_ctx, "345346345645234564536345345345345456534783448567")
                    .ok()
                    .expect("could not create bigint from str");
            let to_str = bigints::to_string_q(q_ctx, &bi_ref)
                .ok()
                .expect("could not tostring bigint");
            assert_eq!(to_str, "345346345645234564536345345345345456534783448567");
            let bi_ref = bigints::new_bigint_i64_q(q_ctx, 659863456456)
                .ok()
                .expect("could not create bigint from u64");
            let to_str = bigints::to_string_q(q_ctx, &bi_ref)
                .ok()
                .expect("could not tostring bigint");
            assert_eq!(to_str, "659863456456");
            let bi_ref = bigints::new_bigint_i64_q(q_ctx, 659863456457)
                .ok()
                .expect("could not create bigint from u64");
            let to_str = bigints::to_string_q(q_ctx, &bi_ref)
                .ok()
                .expect("could not tostring bigint");
            assert_eq!(to_str, "659863456457");
        });
    }
}
