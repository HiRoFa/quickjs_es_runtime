//JS_AtomToCString(ctx: *mut JSContext, atom: JSAtom) -> *const ::std::os::raw::c_char
use crate::eserror::EsError;
use crate::quickjs_utils::primitives;
use crate::quickjsruntime::{OwnedValueRef, QuickJsRuntime};
use libquickjs_sys as q;

pub unsafe fn to_string(q_js_rt: &QuickJsRuntime, atom: &q::JSAtom) -> Result<String, EsError> {
    let val = q::JS_AtomToString(q_js_rt.context, *atom);
    let val_ref = OwnedValueRef::new(val);
    let s = primitives::to_string(q_js_rt, &val_ref);
    s
}
