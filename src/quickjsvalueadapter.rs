//! JSValueRef is a wrapper for quickjs's JSValue. it provides automatic reference counting making it safer to use  

use crate::jsutils::{JsError, JsValueType};
use crate::quickjs_utils::typedarrays::is_typed_array;
use crate::quickjs_utils::{arrays, errors, functions, primitives, promises};
use crate::reflection::is_proxy_instance;
use libquickjs_sys as q;
use std::hash::{Hash, Hasher};
use std::ptr::null_mut;

#[allow(clippy::upper_case_acronyms)]
pub struct QuickJsValueAdapter {
    pub(crate) context: *mut q::JSContext,
    value: q::JSValue,
    ref_ct_decr_on_drop: bool,
    label: String,
}

impl Hash for QuickJsValueAdapter {
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

impl PartialEq for QuickJsValueAdapter {
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

impl Eq for QuickJsValueAdapter {}

impl QuickJsValueAdapter {
    #[allow(dead_code)]
    pub(crate) fn label(&mut self, label: &str) {
        self.label = label.to_string()
    }
}

impl Clone for QuickJsValueAdapter {
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

impl Drop for QuickJsValueAdapter {
    fn drop(&mut self) {
        //log::debug!(
        //    "dropping OwnedValueRef, before free: {}, ref_ct: {}, tag: {}",
        //    self.label,
        //    self.get_ref_count(),
        //    self.value.tag
        //);

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
        //log::trace!("dropping OwnedValueRef, after free",);
    }
}

impl std::fmt::Debug for QuickJsValueAdapter {
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

impl QuickJsValueAdapter {
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

#[cfg(feature = "bellard")]
pub(crate) const TAG_BIG_INT: i64 = -10;
#[cfg(feature = "quickjs-ng")]
pub(crate) const TAG_BIG_INT: i64 = -9;
//pub(crate) const TAG_BIG_FLOAT: i64 = -9;
//pub(crate) const TAG_SYMBOL: i64 = -8;
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

impl QuickJsValueAdapter {
    pub fn is_function(&self) -> bool {
        self.is_object() && self.get_js_type() == JsValueType::Function
    }
    pub fn is_array(&self) -> bool {
        self.is_object() && self.get_js_type() == JsValueType::Array
    }
    pub fn is_error(&self) -> bool {
        self.is_object() && self.get_js_type() == JsValueType::Error
    }
    pub fn is_promise(&self) -> bool {
        self.is_object() && self.get_js_type() == JsValueType::Promise
    }

    pub fn get_js_type(&self) -> JsValueType {
        match self.get_tag() {
            TAG_EXCEPTION => JsValueType::Error,
            TAG_NULL => JsValueType::Null,
            TAG_UNDEFINED => JsValueType::Undefined,
            TAG_BOOL => JsValueType::Boolean,
            TAG_INT => JsValueType::I32,
            TAG_FLOAT64 => JsValueType::F64,
            TAG_STRING => JsValueType::String,
            TAG_OBJECT => {
                // todo get classProto.name and match
                if unsafe { functions::is_function(self.context, self) } {
                    JsValueType::Function
                } else if unsafe { errors::is_error(self.context, self) } {
                    JsValueType::Error
                } else if unsafe { arrays::is_array(self.context, self) } {
                    JsValueType::Array
                } else if unsafe { promises::is_promise(self.context, self) } {
                    JsValueType::Promise
                } else {
                    JsValueType::Object
                }
            }
            TAG_MODULE => todo!(),
            _ => JsValueType::Undefined,
        }
    }

    pub fn is_typed_array(&self) -> bool {
        self.is_object() && unsafe { is_typed_array(self.context, self) }
    }

    pub fn is_proxy_instance(&self) -> bool {
        self.is_object() && unsafe { is_proxy_instance(self.context, self) }
    }

    pub fn type_of(&self) -> &'static str {
        match self.get_tag() {
            TAG_BIG_INT => "bigint",
            TAG_STRING => "string",
            TAG_MODULE => "module",
            TAG_FUNCTION_BYTECODE => "function",
            TAG_OBJECT => {
                if self.get_js_type() == JsValueType::Function {
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

    pub fn to_bool(&self) -> bool {
        if self.get_js_type() == JsValueType::Boolean {
            primitives::to_bool(self).expect("could not convert bool to bool")
        } else {
            panic!("not a boolean");
        }
    }

    pub fn to_i32(&self) -> i32 {
        if self.get_js_type() == JsValueType::I32 {
            primitives::to_i32(self).expect("could not convert to i32")
        } else {
            panic!("not an i32");
        }
    }

    pub fn to_f64(&self) -> f64 {
        if self.get_js_type() == JsValueType::F64 {
            primitives::to_f64(self).expect("could not convert to f64")
        } else {
            panic!("not a f64");
        }
    }

    pub fn to_string(&self) -> Result<String, JsError> {
        match self.get_js_type() {
            JsValueType::I32 => Ok(self.to_i32().to_string()),
            JsValueType::F64 => Ok(self.to_f64().to_string()),
            JsValueType::String => unsafe { primitives::to_string(self.context, self) },
            JsValueType::Boolean => {
                if self.to_bool() {
                    Ok("true".to_string())
                } else {
                    Ok("false".to_string())
                }
            }
            JsValueType::Error => {
                let js_error = unsafe { errors::error_to_js_error(self.context, self) };
                Ok(format!("{js_error}"))
            }
            _ => unsafe { functions::call_to_string(self.context, self) },
        }
    }

    pub fn to_str(&self) -> Result<&str, JsError> {
        if self.get_js_type() == JsValueType::String {
            unsafe { primitives::to_str(self.context, self) }
        } else {
            Err(JsError::new_str("this value is not a string"))
        }
    }
}

#[cfg(test)]
pub mod tests {
    use crate::facades::tests::init_test_rt;
    use crate::jsutils::{JsValueType, Script};

    #[test]
    fn test_to_str() {
        let rt = init_test_rt();
        rt.exe_rt_task_in_event_loop(|q_js_rt| {
            let q_ctx = q_js_rt.get_main_realm();
            let res = q_ctx.eval(Script::new("test_to_str.es", "('hello ' + 'world');"));

            match res {
                Ok(res) => {
                    log::info!("script ran ok: {:?}", res);
                    assert!(res.get_js_type() == JsValueType::String);
                    assert_eq!(res.to_str().expect("str conv failed"), "hello world");
                }
                Err(e) => {
                    log::error!("script failed: {}", e);
                    panic!("script failed");
                }
            }
        });
    }
}
