use crate::quickjsruntime::QuickJsRuntime;
use libquickjs_sys as q;

pub struct JSValueRef {
    value: Option<q::JSValue>,
    label: String,
}

impl JSValueRef {
    pub(crate) fn label(&mut self, label: &str) {
        self.label = label.to_string()
    }
}

impl Clone for JSValueRef {
    fn clone(&self) -> Self {
        Self::new(
            self.value.unwrap(),
            format!("clone of {}", self.label).as_str(),
        )
    }
}

impl Drop for JSValueRef {
    fn drop(&mut self) {
        QuickJsRuntime::do_with(|q_js_rt| unsafe {
            log::debug!("dropping OwnedValueRef, before free: {}", self.label);

            if let Some(value) = self.value {
                // All tags < 0 are garbage collected and need to be freed.
                if value.tag < 0 {
                    // This transmute is OK since if tag < 0, the union will be a refcount
                    // pointer.

                    if self.get_ref_count() < 0 {
                        log::error!(
                            "dropping ref while refcount already 0, which is bad mmkay.. {}",
                            self.label
                        );
                        panic!(
                            "dropping ref while refcount already 0, which is bad mmkay.. {}",
                            self.label
                        );
                    }

                    self.decrement_ref_count();
                    if self.get_ref_count() <= 0 {
                        log::trace!("ref count <= 0, calling __JS_FreeValue");
                        q::__JS_FreeValue(q_js_rt.context, value);
                    }
                }
            }
            log::trace!("dropping OwnedValueRef, after free");
        })
    }
}

impl<'a> std::fmt::Debug for JSValueRef {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self.value.as_ref().unwrap().tag {
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
    pub fn new_no_ref_ct_increment(value: q::JSValue, label: &str) -> Self {
        // todo assert in worker thread
        Self {
            value: Some(value),
            label: label.to_string(),
        }
    }

    pub(crate) fn increment_ref_count(&self) {
        if self.get_tag() < 0 {
            unsafe {
                let ptr = self.value.as_ref().unwrap().u.ptr;
                let p = ptr as *mut q::JSRefCountHeader;
                (*p).ref_count += 1;
            }
        }
    }

    pub(crate) fn decrement_ref_count(&self) {
        if self.get_tag() < 0 {
            unsafe {
                let ptr = self.value.as_ref().unwrap().u.ptr;
                let p = ptr as *mut q::JSRefCountHeader;
                (*p).ref_count -= 1;
            }
        }
    }

    pub fn get_tag(&self) -> i64 {
        self.value.as_ref().unwrap().tag
    }

    pub fn new(value: q::JSValue, label: &str) -> Self {
        let s = Self {
            value: Some(value),
            label: label.to_string(),
        };
        s.increment_ref_count();
        s
    }

    pub fn get_ref_count(&self) -> i32 {
        if self.get_tag() < 0 {
            // This transmute is OK since if tag < 0, the union will be a refcount
            // pointer.
            let ptr = unsafe { self.value.as_ref().unwrap().u.ptr as *mut q::JSRefCountHeader };
            let pref: &mut q::JSRefCountHeader = &mut unsafe { *ptr };
            pref.ref_count
        } else {
            -1
        }
    }

    //pub fn consume_value_decr_rc(self) -> q::JSValue {
    //    self.decrement_ref_count();
    //    self.consume_value_no_decr_rc()
    //}

    pub fn consume_value_no_decr_rc(mut self) -> q::JSValue {
        std::mem::replace(&mut self.value, None).unwrap()
    }

    pub fn borrow_value(&self) -> &q::JSValue {
        self.value.as_ref().unwrap()
    }

    pub fn borrow_value_mut(&mut self) -> &mut q::JSValue {
        self.value.as_mut().unwrap()
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
