//JS_AtomToCString(ctx: *mut JSContext, atom: JSAtom) -> *const ::std::os::raw::c_char
use crate::eserror::EsError;
use crate::quickjs_utils::primitives;
use crate::quickjsruntime::QuickJsRuntime;
use crate::valueref::JSValueRef;
use libquickjs_sys as q;
use std::ffi::CString;

pub struct JSAtomRef {
    atom: q::JSAtom,
}

impl JSAtomRef {
    pub fn new(atom: q::JSAtom) -> Self {
        Self { atom }
    }
    pub(crate) fn get_atom(&self) -> q::JSAtom {
        self.atom
    }
}

impl Drop for JSAtomRef {
    fn drop(&mut self) {
        // free
        QuickJsRuntime::do_with(|q_js_rt| {
            unsafe { q::JS_FreeAtom(q_js_rt.context, self.atom) };
        })
    }
}

pub fn to_string(q_js_rt: &QuickJsRuntime, atom_ref: &JSAtomRef) -> Result<String, EsError> {
    let val = unsafe { q::JS_AtomToString(q_js_rt.context, atom_ref.atom) };
    let val_ref = JSValueRef::new(val, false, true, "atoms::to_string");
    primitives::to_string(q_js_rt, &val_ref)
}

pub fn to_string2(q_js_rt: &QuickJsRuntime, atom: &q::JSAtom) -> Result<String, EsError> {
    let val = unsafe { q::JS_AtomToString(q_js_rt.context, *atom) };
    let val_ref = JSValueRef::new(val, false, true, "atoms::to_string");
    primitives::to_string(q_js_rt, &val_ref)
}

pub fn from_string(q_js_rt: &QuickJsRuntime, string: &str) -> Result<JSAtomRef, EsError> {
    let s = CString::new(string).ok().unwrap();
    let atom = unsafe { q::JS_NewAtomLen(q_js_rt.context, s.as_ptr(), string.len() as u64) };
    Ok(JSAtomRef::new(atom))
}
