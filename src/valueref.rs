use crate::quickjsruntime::QuickJsRuntime;
use libquickjs_sys as q;

pub struct JSValueRef {
    value: q::JSValue,
    label: Option<String>,
}

impl Clone for JSValueRef {
    fn clone(&self) -> Self {
        Self::new(self.value)
    }
}

impl Drop for JSValueRef {
    fn drop(&mut self) {
        QuickJsRuntime::do_with(|q_js_rt| unsafe {
            if self.label.is_some() {
                log::debug!(
                    "dropping OwnedValueRef, before free: {}",
                    self.label.as_ref().unwrap()
                );
            } else {
                log::trace!("dropping OwnedValueRef, before free");
            }
            // All tags < 0 are garbage collected and need to be freed.
            if self.value.tag < 0 {
                // This transmute is OK since if tag < 0, the union will be a refcount
                // pointer.
                let ptr = self.value.u.ptr as *mut q::JSRefCountHeader;
                let pref: &mut q::JSRefCountHeader = &mut *ptr;

                if pref.ref_count <= 0 {
                    if let Some(label) = &self.label {
                        panic!(
                            "dropping ref while refcount already 0, which is bad mmkay.. {}",
                            label
                        );
                    } else {
                        panic!("dropping ref while refcount already 0, which is bad mmkay..");
                    }
                }

                pref.ref_count -= 1;
                if pref.ref_count <= 0 {
                    log::trace!("ref count <= 0, calling __JS_FreeValue");
                    q::__JS_FreeValue(q_js_rt.context, self.value);
                }
            }
            log::trace!("dropping OwnedValueRef, after free");
        })
    }
}

impl<'a> std::fmt::Debug for JSValueRef {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self.value.tag {
            TAG_EXCEPTION => write!(f, "Exception(?)"),
            TAG_NULL => write!(f, "NULL"),
            TAG_UNDEFINED => write!(f, "UNDEFINED"),
            TAG_BOOL => write!(f, "Bool(?)",),
            TAG_INT => write!(f, "Int(?)"),
            TAG_FLOAT64 => write!(f, "Float(?)"),
            TAG_STRING => write!(f, "String(?)"),
            TAG_OBJECT => write!(f, "Object(?)"),
            _ => write!(f, "?"),
        }
    }
}

impl JSValueRef {
    /// create a new OwnedValueRef
    pub fn new_no_ref_ct_increment(value: q::JSValue) -> Self {
        // todo assert in worker thread
        Self { value, label: None }
    }

    pub fn increment_ref_count(&self) {
        if self.value.tag < 0 {
            unsafe {
                let ptr = self.value.u.ptr;
                let p = ptr as *mut q::JSRefCountHeader;
                (*p).ref_count += 1;
            }
        }
    }

    pub fn new(value: q::JSValue) -> Self {
        let s = Self { value, label: None };
        s.increment_ref_count();
        s
    }

    pub fn get_ref_count(&self) -> i32 {
        if self.value.tag < 0 {
            // This transmute is OK since if tag < 0, the union will be a refcount
            // pointer.
            let ptr = unsafe { self.value.u.ptr as *mut q::JSRefCountHeader };
            let pref: &mut q::JSRefCountHeader = &mut unsafe { *ptr };
            pref.ref_count
        } else {
            -1
        }
    }

    pub fn label(&mut self, label: &str) {
        self.label = Some(label.to_string());
    }

    pub fn clone_value_up_rc(&self) -> q::JSValue {
        self.increment_ref_count();
        self.value
    }

    pub fn borrow_value(&self) -> &q::JSValue {
        &self.value
    }

    pub fn borrow_value_mut(&mut self) -> &mut q::JSValue {
        &mut self.value
    }

    pub fn is_null_or_undefined(&self) -> bool {
        self.is_null() || self.is_undefined()
    }

    /// return true if the wrapped value represents a JS null value
    pub fn is_undefined(&self) -> bool {
        self.borrow_value().tag == TAG_UNDEFINED
    }

    /// Get the inner JSValue without freeing in drop.
    ///
    /// Unsafe because the caller is responsible for freeing the value.
    //unsafe fn into_inner(mut self) -> q::JSValue {
    //let v = self.value;
    //self.value = q::JSValue {
    //u: q::JSValueUnion { int32: 0 },
    //tag: TAG_NULL,
    //};
    //v
    //}

    /// return true if the wrapped value represents a JS null value
    pub fn is_null(&self) -> bool {
        self.borrow_value().tag == TAG_NULL
    }

    /// return true if the wrapped value represents a JS boolean value
    pub fn is_bool(&self) -> bool {
        self.borrow_value().tag == TAG_BOOL
    }

    /// return true if the wrapped value represents a JS INT value
    pub fn is_i32(&self) -> bool {
        self.borrow_value().tag == TAG_INT
    }

    /// return true if the wrapped value represents a JS F64 value
    pub fn is_f64(&self) -> bool {
        self.borrow_value().tag == TAG_FLOAT64
    }

    pub fn is_big_int(&self) -> bool {
        self.borrow_value().tag == TAG_BIG_INT
    }

    /// return true if the wrapped value represents a JS Esception value
    pub fn is_exception(&self) -> bool {
        self.borrow_value().tag == TAG_EXCEPTION
    }

    /// return true if the wrapped value represents a JS Object value
    pub fn is_object(&self) -> bool {
        self.borrow_value().tag == TAG_OBJECT
    }

    /// return true if the wrapped value represents a JS String value
    pub fn is_string(&self) -> bool {
        self.borrow_value().tag == TAG_STRING
    }
}

pub(crate) const TAG_BIG_INT: i64 = -10;
pub(crate) const TAG_STRING: i64 = -7;
pub(crate) const TAG_OBJECT: i64 = -1;
pub(crate) const TAG_INT: i64 = 0;
pub(crate) const TAG_BOOL: i64 = 1;
pub(crate) const TAG_NULL: i64 = 2;
pub(crate) const TAG_UNDEFINED: i64 = 3;
pub(crate) const TAG_EXCEPTION: i64 = 6;
pub(crate) const TAG_FLOAT64: i64 = 7;
