use crate::eserror::EsError;
use crate::quickjs_utils::{objects, primitives};
use crate::quickjsruntime::QuickJsRuntime;
use crate::valueref::JSValueRef;
use libquickjs_sys as q;

pub fn new_error(q_js_rt: &QuickJsRuntime, message: &str) -> Result<JSValueRef, EsError> {
    let obj = unsafe { q::JS_NewError(q_js_rt.context) };
    let obj_ref = JSValueRef::new(obj);
    objects::set_property(
        q_js_rt,
        &obj_ref,
        "message",
        &primitives::from_string(q_js_rt, message)?,
    )?;
    Ok(obj_ref)
}

pub fn is_error(q_js_rt: &QuickJsRuntime, obj_ref: &JSValueRef) -> bool {
    if obj_ref.is_object() {
        let res = unsafe { q::JS_IsError(q_js_rt.context, *obj_ref.borrow_value()) };
        res != 0
    } else {
        false
    }
}
