use crate::eserror::EsError;
use crate::quickjs_utils::atoms;
use crate::quickjs_utils::atoms::JSAtomRef;
use libquickjs_sys as q;
use std::os::raw::c_int;

#[allow(clippy::upper_case_acronyms)]
/// this is a wrapper struct for JSPropertyEnum struct in quickjs
/// it used primarily as a result of objects::get_own_property_names()
pub struct JSPropertyEnumRef {
    context: *mut q::JSContext,
    property_enum: *mut q::JSPropertyEnum,
    length: u32,
}

impl JSPropertyEnumRef {
    pub fn new(
        context: *mut q::JSContext,
        property_enum: *mut q::JSPropertyEnum,
        length: u32,
    ) -> Self {
        Self {
            context,
            property_enum,
            length,
        }
    }
    /// get a raw ptr to an Atom
    /// # Safety
    /// do not drop the JSPropertyEnumRef while still using the ptr
    pub unsafe fn get_atom_raw(&self, index: u32) -> *mut q::JSAtom {
        if index >= self.length {
            panic!("index out of bounds");
        }
        let prop: *mut q::JSPropertyEnum = self.property_enum.offset(index as isize);
        let atom: *mut q::JSAtom = (*prop).atom as *mut q::JSAtom;
        atom
    }
    pub fn get_atom(&self, index: u32) -> JSAtomRef {
        let atom: *mut q::JSAtom = unsafe { self.get_atom_raw(index) };
        let atom_ref = JSAtomRef::new(self.context, atom as q::JSAtom);
        atom_ref.increment_ref_ct();
        atom_ref
    }
    pub fn get_name(&self, index: u32) -> Result<String, EsError> {
        let atom: *mut q::JSAtom = unsafe { self.get_atom_raw(index) };
        let atom = atom as q::JSAtom;
        unsafe { Ok(atoms::to_str(self.context, &atom)?.to_string()) }
    }
    pub fn is_enumerable(&self, index: u32) -> bool {
        if index >= self.length {
            panic!("index out of bounds");
        }
        unsafe {
            let prop: *mut q::JSPropertyEnum = self.property_enum.offset(index as isize);
            let is_enumerable: c_int = (*prop).is_enumerable;
            is_enumerable != 0
        }
    }
    pub fn len(&self) -> u32 {
        self.length
    }
    pub fn is_empty(&self) -> bool {
        self.length == 0
    }
}

impl Drop for JSPropertyEnumRef {
    fn drop(&mut self) {
        unsafe {
            for index in 0..self.length {
                let prop: *mut q::JSPropertyEnum = self.property_enum.offset(index as isize);
                q::JS_FreeAtom(self.context, (*prop).atom);
            }

            q::js_free(self.context, self.property_enum as *mut std::ffi::c_void);
        }
    }
}
