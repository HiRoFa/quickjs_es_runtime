//JS_AtomToCString(ctx: *mut JSContext, atom: JSAtom) -> *const ::std::os::raw::c_char
use crate::eserror::EsError;
use crate::quickjs_utils::primitives;
use crate::quickjsruntime::QuickJsRuntime;
use crate::valueref::JSValueRef;
use libquickjs_sys as q;
use std::ffi::CString;

pub fn to_string(q_js_rt: &QuickJsRuntime, atom: &q::JSAtom) -> Result<String, EsError> {
    let val = unsafe { q::JS_AtomToString(q_js_rt.context, *atom) };
    let val_ref = JSValueRef::new(val);
    primitives::to_string(q_js_rt, &val_ref)
}

pub fn from_string(q_js_rt: &QuickJsRuntime, string: &str) -> Result<q::JSAtom, EsError> {
    let s = CString::new(string).ok().unwrap();
    Ok(unsafe { q::JS_NewAtomLen(q_js_rt.context, s.as_ptr(), string.len() as u64) })
}
