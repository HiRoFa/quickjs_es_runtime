//JS_AtomToCString(ctx: *mut JSContext, atom: JSAtom) -> *const ::std::os::raw::c_char
use crate::eserror::EsError;
use crate::quickjs_utils::primitives;
use crate::quickjscontext::QuickJsContext;
use crate::valueref::JSValueRef;
use libquickjs_sys as q;
use std::ffi::CString;

pub struct JSAtomRef {
    context: *mut q::JSContext,
    atom: q::JSAtom,
}

impl JSAtomRef {
    pub fn new(context: *mut q::JSContext, atom: q::JSAtom) -> Self {
        Self { context, atom }
    }
    pub(crate) fn get_atom(&self) -> q::JSAtom {
        self.atom
    }
}

impl Drop for JSAtomRef {
    fn drop(&mut self) {
        // free
        unsafe { q::JS_FreeAtom(self.context, self.atom) };
    }
}

pub fn to_string_q(q_ctx: &QuickJsContext, atom_ref: &JSAtomRef) -> Result<String, EsError> {
    unsafe { to_string(q_ctx.context, atom_ref) }
}

pub unsafe fn to_string(
    context: *mut q::JSContext,
    atom_ref: &JSAtomRef,
) -> Result<String, EsError> {
    let val = q::JS_AtomToString(context, atom_ref.atom);
    let val_ref = JSValueRef::new(context, val, false, true, "atoms::to_string");
    primitives::to_string(context, &val_ref)
}

pub fn to_string2_q(q_ctx: &QuickJsContext, atom: &q::JSAtom) -> Result<String, EsError> {
    unsafe { to_string2(q_ctx.context, atom) }
}

pub unsafe fn to_string2(context: *mut q::JSContext, atom: &q::JSAtom) -> Result<String, EsError> {
    let val = q::JS_AtomToString(context, *atom);
    let val_ref = JSValueRef::new(context, val, false, true, "atoms::to_string");
    primitives::to_string(context, &val_ref)
}

pub fn from_string_q(q_ctx: &QuickJsContext, string: &str) -> Result<JSAtomRef, EsError> {
    unsafe { from_string(q_ctx.context, string) }
}

pub unsafe fn from_string(context: *mut q::JSContext, string: &str) -> Result<JSAtomRef, EsError> {
    let s = CString::new(string).ok().unwrap();

    #[cfg(target_pointer_width = "64")]
    let len = string.len() as u64;
    #[cfg(target_pointer_width = "32")]
    let len = string.len() as u32;

    let atom = q::JS_NewAtomLen(context, s.as_ptr(), len);
    Ok(JSAtomRef::new(context, atom))
}
