// store in thread_local

use crate::eserror::EsError;
use crate::esscript::EsScript;
use crate::quickjs_utils::modules;
use crate::valueref::{JSValueRef, TAG_EXCEPTION};
use libquickjs_sys as q;
use std::cell::RefCell;
use std::ffi::{CString, NulError};

thread_local! {
   /// the thread-local SpiderMonkeyRuntime
   /// this only exists for the worker thread of the EsEventQueue
   pub(crate) static QJS_RT: RefCell<QuickJsRuntime> = RefCell::new(QuickJsRuntime::new());
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
        modules::set_module_loader(&q_rt);
        q_rt.set_promise_rejection_tracker();

        q_rt
    }

    pub fn gc(&self) {}

    pub fn eval(&self, script: EsScript) -> Result<JSValueRef, EsError> {
        let filename_c =
            make_cstring(script.get_path()).expect("failed to create c_string from path");
        let code_c = make_cstring(script.get_code()).expect("failed to create c_string from code");

        log::debug!("q_js_rt.eval file {}", script.get_path());

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
        let ret = JSValueRef::new(value_raw);
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

    pub fn eval_module(&self, script: EsScript) -> Result<JSValueRef, EsError> {
        let filename_c =
            make_cstring(script.get_path()).expect("failed to create c_string from path");
        let code_c = make_cstring(script.get_code()).expect("failed to create c_string from code");

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
        let ret = JSValueRef::new(value_raw);
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

    /// throw an internal error to quickjs and create a new ex obj
    pub fn report_ex(&self, err: &str) -> q::JSValue {
        let c_err = CString::new(err);
        unsafe { q::JS_ThrowInternalError(self.context, c_err.as_ref().ok().unwrap().as_ptr()) };
        q::JSValue {
            u: q::JSValueUnion { int32: 0 },
            tag: TAG_EXCEPTION,
        }
    }

    /// Get the last exception from the runtime, and if present, convert it to a ExceptionError.
    pub fn get_exception(&self) -> Option<EsError> {
        let raw = unsafe { q::JS_GetException(self.context) };
        let value = JSValueRef::new(raw);

        if value.is_null() {
            None
        } else {
            let err = if value.is_exception() {
                EsError::new_str("Could not get exception from runtime")
            } else if value.is_object() {
                // todo figure out how to get lineno/col/filename etc
                match crate::quickjs_utils::functions::call_to_string(self, &value) {
                    Ok(strval) => {
                        if strval.contains("out of memory") {
                            EsError::new_str("out of memory")
                        } else {
                            EsError::new_string(strval)
                        }
                    }
                    Err(_) => EsError::new_str("Unknown exception2"),
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
}

unsafe extern "C" fn promise_rejection_tracker(
    _ctx: *mut q::JSContext,
    _promise: q::JSValue,
    _reason: q::JSValue,
    is_handled: ::std::os::raw::c_int,
    _opaque: *mut ::std::os::raw::c_void,
) {
    if is_handled == 0 {
        log::error!("unhandled promise rejection detected");
    }
}

impl Drop for QuickJsRuntime {
    fn drop(&mut self) {
        unsafe {
            q::JS_FreeContext(self.context);
            q::JS_FreeRuntime(self.runtime);
        }
    }
}

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
        rt.eval(EsScript::new("test.es", "1+1;"))
            .ok()
            .expect("could not eval");
    }
}
