use crate::eserror::EsError;
use crate::quickjsruntime::{OwnedValueRef, QuickJsRuntime};
use libquickjs_sys as q;

pub fn call_to_string(
    q_js_rt: &QuickJsRuntime,
    obj_ref: &OwnedValueRef,
) -> Result<String, EsError> {
    if obj_ref.is_string() {
        crate::quickjs_utils::primitives::to_string(q_js_rt, obj_ref)
    } else {
        log::trace!("calling JS_ToString on a {}", obj_ref.value.tag);

        let res = unsafe { q::JS_ToString(q_js_rt.context, obj_ref.value) };
        let res_ref = OwnedValueRef::new(res);

        log::trace!("called JS_ToString got a {}", res_ref.value.tag);

        if !res_ref.is_string() {
            return Err(EsError::new_str("Could not convert value to string"));
        }
        crate::quickjs_utils::primitives::to_string(q_js_rt, &res_ref)
    }
}
