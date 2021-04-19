use crate::quickjs_utils::atoms::JSAtomRef;
use libquickjs_sys as q;

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
        return atom_ref;
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
