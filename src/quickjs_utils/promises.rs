use crate::eserror::EsError;
use crate::quickjs_utils::get_global;
use crate::quickjs_utils::objects::{get_property, is_instance_of_by_name};
use crate::quickjsruntime::{OwnedValueRef, QuickJsRuntime};
use libquickjs_sys as q;

pub fn is_promise(q_js_rt: &QuickJsRuntime, obj_ref: &OwnedValueRef) -> Result<bool, EsError> {
    is_instance_of_by_name(q_js_rt, obj_ref, "Promise")
}
