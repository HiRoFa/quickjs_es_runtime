// store in thread_local

use crate::eserror::EsError;
use crate::esscript::EsScript;
use libquickjs_sys as q;
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
    fn new() -> Self {
        log::trace!("creating new QuickJsRuntime");
        let runtime = unsafe { q::JS_NewRuntime() };
        if runtime.is_null() {
            panic!("RuntimeCreationFailed");
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
            panic!("ContextCreationFailed");
        }

        // Initialize the promise resolver helper code.
        // This code is needed by Self::resolve_value
        let q_rt = Self { runtime, context };

        // test like this, impl later
        q_rt.set_module_loader();
        q_rt.set_promise_rejection_tracker();

        q_rt
    }

    pub fn gc(&self) {}

    pub fn eval(&self, script: EsScript) -> Result<OwnedValueRef, EsError> {
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
        let ret = OwnedValueRef::new(value_raw);
        if ret.is_exception() {
            let ex_opt = self.get_exception();
            if let Some(ex) = ex_opt {
                Err(ex)
            } else {
                Err(EsError::new_str("eval failed and could not get exception"))
            }
        } else {
            Ok(ret)
        }
    }

    pub fn eval_module(&self, script: EsScript) -> Result<OwnedValueRef, EsError> {
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

        // check for error
        let ret = OwnedValueRef::new(value_raw);
        if ret.is_exception() {
            let ex_opt = self.get_exception();
            if let Some(ex) = ex_opt {
                Err(ex)
            } else {
                Err(EsError::new_str(
                    "eval_module failed and could not get exception",
                ))
            }
        } else {
            Ok(ret)
        }
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
    pub(crate) fn get_exception(&self) -> Option<EsError> {
        let raw = unsafe { q::JS_GetException(self.context) };
        let value = OwnedValueRef::new(raw);

        if value.is_null() {
            None
        } else {
            let err = if value.is_exception() {
                EsError::new_str("Could not get exception from runtime".into())
            } else if value.is_object() {
                // todo figure out how to get lineno/col/filename etc
                match crate::quickjs_utils::functions::call_to_string(self, &value) {
                    Ok(strval) => {
                        if strval.contains("out of memory") {
                            EsError::new_str("out of memory".into())
                        } else {
                            EsError::new_string(strval)
                        }
                    }
                    Err(_) => EsError::new_str("Unknown exception2".into()),
                }
            } else {
                EsError::new_str("no clue what happended")
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
    value: Option<q::JSValue>,
    no_free: bool,
}

impl Drop for OwnedValueRef {
    fn drop(&mut self) {
        log::trace!("dropping OwnedValueRef, isSome={}", self.value.is_some());
        if !self.no_free && self.value.is_some() {
            QuickJsRuntime::do_with(|q_js_rt| unsafe {
                free_value(q_js_rt.context, self.consume_value());
            })
        }
    }
}

impl<'a> std::fmt::Debug for OwnedValueRef {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self.borrow_value().tag {
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
        Self {
            value: Some(value),
            no_free: false,
        }
    }

    pub fn new_no_free(value: q::JSValue) -> Self {
        // todo assert in worker thread
        Self {
            value: Some(value),
            no_free: true,
        }
    }

    pub fn borrow_value(&self) -> &q::JSValue {
        self.value.as_ref().expect("OwnedValueRef was consumed")
    }

    pub fn consume_value(&mut self) -> q::JSValue {
        std::mem::replace(&mut self.value, None).unwrap()
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

#[cfg(test)]
pub mod tests {
    use crate::esscript::EsScript;
    use crate::quickjsruntime::QuickJsRuntime;

    #[test]
    fn test_rt() {
        let rt = QuickJsRuntime::new();
        rt.eval(EsScript::new("test.es".to_string(), "1+1;".to_string()))
            .ok()
            .expect("could not eval");
    }
}
