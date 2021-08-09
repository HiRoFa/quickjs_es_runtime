use crate::quickjs_utils::{functions, primitives};
use crate::quickjsruntimeadapter::QuickJsRuntimeAdapter;
use hirofa_utils::js_utils::adapters::JsValueAdapter;
use hirofa_utils::js_utils::facades::JsValueType;
use hirofa_utils::js_utils::JsError;
use libquickjs_sys as q;
use std::hash::{Hash, Hasher};
use std::ptr::null_mut;

#[allow(clippy::upper_case_acronyms)]
pub struct JSValueRef {
    pub(crate) context: *mut q::JSContext,
    value: q::JSValue,
    ref_ct_decr_on_drop: bool,
    label: String,
}

impl Hash for JSValueRef {
    fn hash<H: Hasher>(&self, state: &mut H) {
        let self_u = self.value.u;
        if self.is_i32() || self.is_bool() {
            unsafe { self_u.int32.hash(state) };
        } else if self.is_f64() {
            unsafe { (self_u.float64 as i32).hash(state) };
        } else {
            unsafe { self_u.ptr.hash(state) };
        }
    }
}

impl PartialEq for JSValueRef {
    fn eq(&self, other: &Self) -> bool {
        if self.get_tag() != other.get_tag() {
            false
        } else {
            let self_u = self.value.u;
            let other_u = other.value.u;
            unsafe {
                self_u.int32 == other_u.int32
                    && self_u.float64 == other_u.float64
                    && self_u.ptr == other_u.ptr
            }
        }
    }
}

impl Eq for JSValueRef {}

impl JSValueRef {
    #[allow(dead_code)]
    pub(crate) fn label(&mut self, label: &str) {
        self.label = label.to_string()
    }
}

impl Clone for JSValueRef {
    fn clone(&self) -> Self {
        Self::new(
            self.context,
            self.value,
            true,
            true,
            format!("clone of {}", self.label).as_str(),
        )
    }
}

impl Drop for JSValueRef {
    fn drop(&mut self) {
        log::debug!(
            "dropping OwnedValueRef, before free: {}, ref_ct: {}, tag: {}",
            self.label,
            self.get_ref_count(),
            self.value.tag
        );

        // All tags < 0 are garbage collected and need to be freed.
        if self.value.tag < 0 {
            // This transmute is OK since if tag < 0, the union will be a refcount
            // pointer.

            if self.ref_ct_decr_on_drop {
                if self.get_ref_count() <= 0 {
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
            }
        }
        log::trace!("dropping OwnedValueRef, after free",);
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
            TAG_MODULE => write!(f, "Module(?)"),
            _ => write!(f, "?"),
        }
    }
}

impl JSValueRef {
    pub(crate) fn increment_ref_count(&self) {
        if self.get_tag() < 0 {
            unsafe { libquickjs_sys::JS_DupValue(self.context, *self.borrow_value()) }
        }
    }

    pub(crate) fn decrement_ref_count(&self) {
        if self.get_tag() < 0 {
            unsafe { libquickjs_sys::JS_FreeValue(self.context, *self.borrow_value()) }
        }
    }

    pub fn get_tag(&self) -> i64 {
        self.value.tag
    }

    pub fn new_no_context(value: q::JSValue, label: &str) -> Self {
        Self {
            context: null_mut(),
            value,
            ref_ct_decr_on_drop: false,
            label: label.to_string(),
        }
    }

    pub fn new(
        context: *mut q::JSContext,
        value: q::JSValue,
        ref_ct_incr: bool,
        ref_ct_decr_on_drop: bool,
        label: &str,
    ) -> Self {
        debug_assert!(!label.is_empty());

        let s = Self {
            context,
            value,
            ref_ct_decr_on_drop,
            label: label.to_string(),
        };
        if ref_ct_incr {
            s.increment_ref_count();
        }
        s
    }

    pub fn get_ref_count(&self) -> i32 {
        if self.get_tag() < 0 {
            // This transmute is OK since if tag < 0, the union will be a refcount
            // pointer.
            let ptr = unsafe { self.value.u.ptr as *mut q::JSRefCountHeader };
            let pref: &mut q::JSRefCountHeader = &mut unsafe { *ptr };
            pref.ref_count
        } else {
            -1
        }
    }

    /// borrow the value but first increment the refcount, this is useful for when the value is returned or passed to functions
    pub fn clone_value_incr_rc(&self) -> q::JSValue {
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
        unsafe { q::JS_IsUndefined(self.value) }
    }

    /// return true if the wrapped value represents a JS null value
    pub fn is_null(&self) -> bool {
        unsafe { q::JS_IsNull(self.value) }
    }

    /// return true if the wrapped value represents a JS boolean value
    pub fn is_bool(&self) -> bool {
        unsafe { q::JS_IsBool(self.value) }
    }

    /// return true if the wrapped value represents a JS INT value
    pub fn is_i32(&self) -> bool {
        // todo figure out diff between i32/f64/Number
        // unsafe { q::JS_IsNumber(self.borrow_value()) }
        self.borrow_value().tag == TAG_INT
    }

    /// return true if the wrapped value represents a Module
    pub fn is_module(&self) -> bool {
        self.borrow_value().tag == TAG_MODULE
    }

    /// return true if the wrapped value represents a compiled function
    pub fn is_compiled_function(&self) -> bool {
        self.borrow_value().tag == TAG_FUNCTION_BYTECODE
    }

    /// return true if the wrapped value represents a JS F64 value
    pub fn is_f64(&self) -> bool {
        self.borrow_value().tag == TAG_FLOAT64
    }

    pub fn is_big_int(&self) -> bool {
        // unsafe { q::JS_IsBigInt(ctx, self.borrow_value()) }
        self.borrow_value().tag == TAG_BIG_INT
    }

    /// return true if the wrapped value represents a JS Exception value
    pub fn is_exception(&self) -> bool {
        unsafe { q::JS_IsException(self.value) }
    }

    /// return true if the wrapped value represents a JS Object value
    pub fn is_object(&self) -> bool {
        unsafe { q::JS_IsObject(self.value) }
    }

    /// return true if the wrapped value represents a JS String value
    pub fn is_string(&self) -> bool {
        unsafe { q::JS_IsString(self.value) }
    }
}

pub(crate) const TAG_BIG_INT: i64 = -10;
pub(crate) const TAG_STRING: i64 = -7;
pub(crate) const TAG_MODULE: i64 = -3;
pub(crate) const TAG_FUNCTION_BYTECODE: i64 = -2;
pub(crate) const TAG_OBJECT: i64 = -1;
pub(crate) const TAG_INT: i64 = 0;
pub(crate) const TAG_BOOL: i64 = 1;
pub(crate) const TAG_NULL: i64 = 2;
pub(crate) const TAG_UNDEFINED: i64 = 3;
pub(crate) const TAG_EXCEPTION: i64 = 6;
pub(crate) const TAG_FLOAT64: i64 = 7;

impl JsValueAdapter for JSValueRef {
    type JsRuntimeAdapterType = QuickJsRuntimeAdapter;

    fn js_get_type(&self) -> JsValueType {
        match self.get_tag() {
            TAG_EXCEPTION => todo!(),
            TAG_NULL => JsValueType::Null,
            TAG_UNDEFINED => JsValueType::Undefined,
            TAG_BOOL => JsValueType::Boolean,
            TAG_INT => JsValueType::I32,
            TAG_FLOAT64 => JsValueType::F64,
            TAG_STRING => JsValueType::String,
            TAG_OBJECT => {
                if unsafe { functions::is_function(self.context, self) } {
                    JsValueType::Function
                } else {
                    JsValueType::Object
                }
            }
            TAG_MODULE => todo!(),
            _ => JsValueType::Undefined,
        }
    }

    fn js_type_of(&self) -> &'static str {
        match self.get_tag() {
            TAG_BIG_INT => "bigint",
            TAG_STRING => "string",
            TAG_MODULE => "module",
            TAG_FUNCTION_BYTECODE => "function",
            TAG_OBJECT => {
                if self.js_get_type() == JsValueType::Function {
                    "function"
                } else {
                    "object"
                }
            }
            TAG_INT => "number",
            TAG_BOOL => "boolean",
            TAG_NULL => "object",
            TAG_UNDEFINED => "undefined",
            TAG_EXCEPTION => "object",
            TAG_FLOAT64 => "number",
            _ => "unknown",
        }
    }

    fn js_to_bool(&self) -> bool {
        if self.js_get_type() == JsValueType::Boolean {
            primitives::to_bool(self)
                .ok()
                .expect("could not convert bool to bool")
        } else {
            panic!("not a boolean");
        }
    }

    fn js_to_i32(&self) -> i32 {
        if self.js_get_type() == JsValueType::I32 {
            primitives::to_i32(self)
                .ok()
                .expect("could not convert to i32")
        } else {
            panic!("not an i32");
        }
    }

    fn js_to_f64(&self) -> f64 {
        if self.js_get_type() == JsValueType::F64 {
            primitives::to_f64(self)
                .ok()
                .expect("could not convert to f64")
        } else {
            panic!("not a f64");
        }
    }

    fn js_to_string(&self) -> Result<String, JsError> {
        if self.js_get_type() == JsValueType::String {
            unsafe { primitives::to_string(self.context, self) }
        } else {
            unsafe { functions::call_to_string(self.context, self) }
        }
    }
}
