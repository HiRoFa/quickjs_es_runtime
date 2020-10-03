use crate::eserror::EsError;
use crate::quickjs_utils;
use crate::quickjs_utils::{functions, primitives};
use crate::quickjsruntime::QuickJsRuntime;
use crate::valueref::JSValueRef;
use libquickjs_sys as q;

#[allow(dead_code)]
pub fn new_bigint_i64(q_js_rt: &QuickJsRuntime, int: i64) -> Result<JSValueRef, EsError> {
    let res_val = unsafe { q::JS_NewBigInt64(q_js_rt.context, int) };
    let ret = JSValueRef::new_no_ref_ct_increment(res_val);
    assert_eq!(ret.get_ref_count(), 1);
    Ok(ret)
}

#[allow(dead_code)]
pub fn new_bigint_u64(q_js_rt: &QuickJsRuntime, int: u64) -> Result<JSValueRef, EsError> {
    let res_val = unsafe { q::JS_NewBigUint64(q_js_rt.context, int) };
    let ret = JSValueRef::new_no_ref_ct_increment(res_val);
    assert_eq!(ret.get_ref_count(), 1);
    Ok(ret)
}

#[allow(dead_code)]
pub fn new_bigint_str(q_js_rt: &QuickJsRuntime, input_str: &str) -> Result<JSValueRef, EsError> {
    let global_ref = quickjs_utils::get_global(q_js_rt);
    let str_ref = primitives::from_string(q_js_rt, input_str)?;
    let bigint_ref = functions::invoke_member_function(q_js_rt, &global_ref, "BigInt", &[str_ref])?;
    let ret = bigint_ref;
    assert_eq!(ret.get_ref_count(), 1);
    Ok(ret)
}

#[allow(dead_code)]
pub fn to_string(q_js_rt: &QuickJsRuntime, big_int_ref: &JSValueRef) -> Result<String, EsError> {
    if !big_int_ref.is_big_int() {
        return Err(EsError::new_str("big_int_ref was not a big_int"));
    }
    functions::call_to_string(q_js_rt, big_int_ref)
}

#[cfg(test)]
pub mod tests {
    use crate::esruntime::EsRuntime;
    use crate::quickjs_utils::bigints;
    use crate::quickjs_utils::bigints::new_bigint_str;
    use std::sync::Arc;

    #[test]
    fn test_bigint() {
        let rt: Arc<EsRuntime> = crate::esruntime::tests::TEST_ESRT.clone();
        rt.add_to_event_queue_sync(|q_js_rt| {
            let bi_ref =
                new_bigint_str(q_js_rt, "345346345645234564536345345345345456534783448567")
                    .ok()
                    .expect("could not create bigint from str");
            let to_str = bigints::to_string(q_js_rt, &bi_ref)
                .ok()
                .expect("could not tostring bigint");
            assert_eq!(to_str, "345346345645234564536345345345345456534783448567");
            let bi_ref = bigints::new_bigint_u64(q_js_rt, 659863456456)
                .ok()
                .expect("could not create bigint from u64");
            let to_str = bigints::to_string(q_js_rt, &bi_ref)
                .ok()
                .expect("could not tostring bigint");
            assert_eq!(to_str, "659863456456");
            let bi_ref = bigints::new_bigint_i64(q_js_rt, 659863456457)
                .ok()
                .expect("could not create bigint from u64");
            let to_str = bigints::to_string(q_js_rt, &bi_ref)
                .ok()
                .expect("could not tostring bigint");
            assert_eq!(to_str, "659863456457");
        });
    }
}
