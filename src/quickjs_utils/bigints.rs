use crate::eserror::EsError;
use crate::quickjs_utils::functions;
use crate::quickjsruntime::QuickJsRuntime;
use crate::valueref::JSValueRef;

#[allow(dead_code)]
pub fn new_bigint_i64(_q_js_rt: &QuickJsRuntime, _int: i64) -> Result<JSValueRef, EsError> {
    // unsafe { q::JS_NewBigInt64(context, int) },
    unimplemented!();
}

#[allow(dead_code)]
pub fn new_bigint_u64(_q_js_rt: &QuickJsRuntime, _int: u64) -> Result<JSValueRef, EsError> {
    // unsafe { q::JS_NewBigUint64(context, int) },
    unimplemented!();
}

#[allow(dead_code)]
pub fn to_i64(_q_js_rt: &QuickJsRuntime, big_int_ref: &JSValueRef) -> Result<i64, EsError> {
    if !big_int_ref.is_big_int() {
        return Err(EsError::new_str("big_int_ref was not a big_int"));
    }
    //  let ret = unsafe { q::JS_ToBigInt64(context, &mut int, *r) };
    unimplemented!();
}

#[allow(dead_code)]
pub fn to_string(q_js_rt: &QuickJsRuntime, big_int_ref: &JSValueRef) -> Result<String, EsError> {
    if !big_int_ref.is_big_int() {
        return Err(EsError::new_str("big_int_ref was not a big_int"));
    }
    functions::call_to_string(q_js_rt, big_int_ref)
}
