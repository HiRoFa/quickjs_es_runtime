// store in thread_local

use crate::eserror::EsError;
use crate::esscript::EsScript;
use crate::esvalue::EsValueFacade;
use libquickjs_sys as q;
use libquickjs_sys::JSValue as JSVal;
use std::cell::RefCell;
use std::ffi::{CString, NulError};

thread_local! {
   /// the thread-local SpiderMonkeyRuntime
   /// this only exists for the worker thread of the EsEventQueue
   pub(crate) static QJS_RT: RefCell<QuickJsRuntime> = RefCell::new(QuickJsRuntime::new());
}

/// Free a JSValue.
/// This function is the equivalent of JS_FreeValue from quickjs, which can not
/// be used due to being `static inline`.
pub(crate) unsafe fn free_value(context: *mut q::JSContext, value: q::JSValue) {
    // All tags < 0 are garbage collected and need to be freed.
    if value.tag < 0 {
        // This transmute is OK since if tag < 0, the union will be a refcount
        // pointer.
        let ptr = value.u.ptr as *mut q::JSRefCountHeader;
        let pref: &mut q::JSRefCountHeader = &mut *ptr;
        pref.ref_count -= 1;
        if pref.ref_count <= 0 {
            q::__JS_FreeValue(context, value);
        }
    }
}

pub struct QuickJsRuntime {
    pub(crate) runtime: *mut q::JSRuntime,
    pub(crate) context: *mut q::JSContext,
}

impl QuickJsRuntime {
    fn new() -> Result<Self, &'static str> {
        let runtime = unsafe { q::JS_NewRuntime() };
        if runtime.is_null() {
            return Err("RuntimeCreationFailed");
        }

        // Configure memory limit if specified.
        //let memory_limit = None;
        //if let Some(limit) = memory_limit {
        //  unsafe {
        //q::JS_SetMemoryLimit(runtime, limit as _);
        //}
        //}

        let context = unsafe { q::JS_NewContext(runtime) };
        if context.is_null() {
            unsafe {
                q::JS_FreeRuntime(runtime);
            }
            return Err("ContextCreationFailed");
        }

        // Initialize the promise resolver helper code.
        // This code is needed by Self::resolve_value
        let q_rt = Self { runtime, context };

        // test like this, impl later
        q_rt.set_module_loader();
        q_rt.set_promise_rejection_tracker();

        Ok(q_rt)
    }

    pub fn gc(&self) {}

    pub fn eval(&self, script: EsScript) -> Result<JSVal, EsError> {
        let filename_c = make_cstring(script.get_path())
            .ok()
            .expect("failed to create c_string from path");
        let code_c = make_cstring(script.get_code())
            .ok()
            .expect("failed to create c_string from code");

        let value_raw = unsafe {
            q::JS_Eval(
                self.context,
                code_c.as_ptr(),
                script.get_code().len() as _,
                filename_c.as_ptr(),
                q::JS_EVAL_TYPE_GLOBAL as i32,
            )
        };

        // check for error

        Ok(value_raw)
    }

    pub fn eval_module(&self, script: EsScript) -> Result<JSVal, EsError> {
        let filename_c = make_cstring(script.get_path())
            .ok()
            .expect("failed to create c_string from path");
        let code_c = make_cstring(script.get_code())
            .ok()
            .expect("failed to create c_string from code");

        let value_raw = unsafe {
            q::JS_Eval(
                self.context,
                code_c.as_ptr(),
                script.get_code().len() as _,
                filename_c.as_ptr(),
                q::JS_EVAL_TYPE_MODULE as i32,
            )
        };

        // check for error

        Ok(value_raw)
    }

    pub(crate) fn do_with<C, R>(task: C) -> R
    where
        C: FnOnce(&QuickJsRuntime) -> R,
    {
        QJS_RT.with(|qjs_rc| {
            let qjs_rt = &*qjs_rc.borrow();
            task(qjs_rt)
        })
    }

    /// Get the last exception from the runtime, and if present, convert it to a ExceptionError.
    fn get_exception(&self) -> Option<EsError> {
        let raw = unsafe { q::JS_GetException(self.context) };
        let value = OwnedValueRef::new(raw);

        if value.is_null() {
            None
        } else {
            let err = if value.is_exception() {
                EsError::new_str("Could get exception from runtime".into())
            } else {
                match value.to_string() {
                    Ok(strval) => {
                        if strval.contains("out of memory") {
                            EsError::new_str("out of memory".into())
                        } else {
                            EsError::new_string(strval)
                        }
                    }
                    Err(_) => EsError::new_str("Unknown exception".into()),
                }
            };
            Some(err)
        }
    }

    pub fn has_pending_jobs(&self) -> bool {
        let flag = unsafe { q::JS_IsJobPending(self.runtime) };
        flag > 0
    }

    pub fn run_pending_job(&self) -> Result<(), EsError> {
        let flag = unsafe {
            let wrapper_mut = self as *const Self as *mut Self;
            let ctx_mut = &mut (*wrapper_mut).context;
            q::JS_ExecutePendingJob(self.runtime, ctx_mut)
        };
        if flag < 0 {
            let e = self
                .get_exception()
                .unwrap_or_else(|| EsError::new_str("Unknown exception while running pending job"));
            return Err(e);
        }
        Ok(())
    }

    pub fn set_promise_rejection_tracker(&self) {
        let tracker: q::JSHostPromiseRejectionTracker = Some(promise_rejection_tracker);

        unsafe {
            q::JS_SetHostPromiseRejectionTracker(self.runtime, tracker, std::ptr::null_mut());
        }
    }

    #[allow(dead_code)]
    pub fn set_module_loader(&self) {
        log::trace!("setting up module loader");

        let module_normalize: q::JSModuleNormalizeFunc = Some(js_module_normalize);
        let module_loader: q::JSModuleLoaderFunc = Some(js_module_loader);

        let opaque = std::ptr::null_mut();

        unsafe { q::JS_SetModuleLoaderFunc(self.runtime, module_normalize, module_loader, opaque) }
    }
}

pub struct OwnedValueRef {
    pub(crate) value: q::JSValue,
}

impl Drop for OwnedValueRef {
    fn drop(&mut self) {
        QuickJsRuntime::do_with(|q_js_rt| unsafe {
            free_value(q_js_rt.context, self.value);
        })
    }
}

impl<'a> std::fmt::Debug for OwnedValueRef {
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

impl OwnedValueRef {
    /// create a new OwnedValueRef
    pub fn new(value: q::JSValue) -> Self {
        // todo assert in worker thread
        Self { value }
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
        self.value.tag == TAG_NULL
    }

    /// return true if the wrapped value represents a JS boolean value
    pub fn is_bool(&self) -> bool {
        self.value.tag == TAG_BOOL
    }

    /// return true if the wrapped value represents a JS Esception value
    pub fn is_exception(&self) -> bool {
        self.value.tag == TAG_EXCEPTION
    }

    /// return true if the wrapped value represents a JS Object value
    pub fn is_object(&self) -> bool {
        self.value.tag == TAG_OBJECT
    }

    /// return true if the wrapped value represents a JS String value
    pub fn is_string(&self) -> bool {
        self.value.tag == TAG_STRING
    }

    /// convert the value to a String
    pub fn to_string(&self) -> Result<String, EsError> {
        let value = if self.is_string() {
            Ok(self
                .to_value()
                .ok()
                .expect("could not create string")
                .get_str()
                .to_string())
        } else {
            QuickJsRuntime::do_with(|q_js_rt| {
                let raw = unsafe { q::JS_ToString(q_js_rt.context, self.value) };
                let value = OwnedValueRef::new(raw);

                if value.value.tag != TAG_STRING {
                    return Err(EsError::new_str("Could not convert value to string".into()));
                }
                Ok(value
                    .to_value()
                    .ok()
                    .expect("Could not convert value to string")
                    .get_str()
                    .to_string())
            })
        };

        value
    }

    /// return true if the wrapped value represents a JsValue
    pub fn to_value(&self) -> Result<EsValueFacade, EsError> {
        QuickJsRuntime::do_with(|q_js_rt| EsValueFacade::from_jsval(q_js_rt, self))
    }

    /// return true if the wrapped value represents a bool
    pub fn to_bool(&self) -> Result<bool, EsError> {
        let esvf = self.to_value().ok().unwrap();
        if esvf.is_boolean() {
            Ok(esvf.get_boolean())
        } else {
            Err(EsError::new_str("not a boolean value"))
        }
    }
}

/// Wraps an object from the quickjs runtime.
/// Provides convenience property accessors.
pub struct OwnedObjectRef {
    value: OwnedValueRef,
}

#[allow(missing_docs)]
impl OwnedObjectRef {
    pub fn new(value: OwnedValueRef) -> Result<Self, EsError> {
        if value.value.tag != TAG_OBJECT {
            Err(EsError::new_str("Expected an object"))
        } else {
            Ok(Self { value })
        }
    }

    fn into_value(self) -> OwnedValueRef {
        self.value
    }

    /// Get the tag of a property.
    fn property_tag(&self, name: &str) -> Result<i64, EsError> {
        let cname = make_cstring(name)
            .ok()
            .expect("could not convert to cstring");
        let raw = unsafe {
            q::JS_GetPropertyStr(self.value.context.context, self.value.value, cname.as_ptr())
        };
        let t = raw.tag;
        unsafe {
            free_value(self.value.context.context, raw);
        }
        Ok(t)
    }

    /// Determine if the object is a promise by checking the presence of
    /// a 'then' and a 'catch' property.
    fn is_promise(&self) -> Result<bool, EsError> {
        if self.property_tag("then")? == TAG_OBJECT && self.property_tag("catch")? == TAG_OBJECT {
            Ok(true)
        } else {
            Ok(false)
        }
    }

    pub fn property(&self, name: &str) -> Result<OwnedValueRef, EsError> {
        let cname = make_cstring(name)?;
        let raw = unsafe {
            q::JS_GetPropertyStr(self.value.context.context, self.value.value, cname.as_ptr())
        };

        if raw.tag == TAG_EXCEPTION {
            Err(EsError::new_string(format!(
                "Exception while getting property '{}'",
                name
            )))
        } else if raw.tag == TAG_UNDEFINED {
            Err(EsError::new_string(format!(
                "Property '{}' not found",
                name
            )))
        } else {
            Ok(OwnedValueRef::new(raw))
        }
    }

    unsafe fn set_property_raw(&self, name: &str, value: q::JSValue) -> Result<(), EsError> {
        let cname = make_cstring(name)?;
        let ret = q::JS_SetPropertyStr(
            self.value.context.context,
            self.value.value,
            cname.as_ptr(),
            value,
        );
        if ret < 0 {
            Err(EsError::new_str("Could not set property"))
        } else {
            Ok(())
        }
    }

    // pub fn set_property(&self, name: &str, value: JsValue) -> Result<(), ExecutionError> {
    //     let qval = self.value.context.serialize_value(value)?;
    //     unsafe { self.set_property_raw(name, qval.value) }
    // }
}

unsafe extern "C" fn promise_rejection_tracker(
    _ctx: *mut q::JSContext,
    _promise: q::JSValue,
    _reason: q::JSValue,
    _is_handled: ::std::os::raw::c_int,
    _opaque: *mut ::std::os::raw::c_void,
) {
    log::error!("promise rejected");
}

unsafe extern "C" fn js_module_normalize(
    _ctx: *mut q::JSContext,
    _module_base_name: *const ::std::os::raw::c_char,
    _module_name: *const ::std::os::raw::c_char,
    _opaque: *mut ::std::os::raw::c_void,
) -> *mut ::std::os::raw::c_char {
    // todo

    log::trace!("js_module_normalize called");
    std::ptr::null_mut()
}

unsafe extern "C" fn js_module_loader(
    _ctx: *mut q::JSContext,
    _module_name: *const ::std::os::raw::c_char,
    _opaque: *mut ::std::os::raw::c_void,
) -> *mut q::JSModuleDef {
    //todo
    log::trace!("js_module_loader called");
    std::ptr::null_mut()
}

impl Drop for QuickJsRuntime {
    fn drop(&mut self) {
        unsafe {
            q::JS_FreeContext(self.context);
            q::JS_FreeRuntime(self.runtime);
        }
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

/// Helper for creating CStrings.
pub(crate) fn make_cstring(value: impl Into<Vec<u8>>) -> Result<CString, NulError> {
    CString::new(value)
}

/// Helper to construct null JsValue
fn js_null_value() -> q::JSValue {
    q::JSValue {
        u: q::JSValueUnion { int32: 0 },
        tag: TAG_NULL,
    }
}

#[cfg(test)]
pub mod tests {
    use crate::esscript::EsScript;
    use crate::quickjsruntime::QuickJsRuntime;

    #[test]
    fn test_rt() {
        let rt = QuickJsRuntime::new().ok().expect("whoops");
        rt.eval(EsScript::new("test.es".to_string(), "1+1;".to_string()));
    }
}
