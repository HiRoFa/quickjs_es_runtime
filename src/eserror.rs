use std::fmt::{Error, Formatter};

pub struct EsError {
    name: String,
    message: String,
    stack: String,
}

impl EsError {
    pub fn new(name: String, message: String, stack: String) -> Self {
        Self {
            name,
            message,
            stack,
        }
    }
    pub fn new_str(err: &str) -> Self {
        Self::new_string(err.to_string())
    }
    pub fn new_string(err: String) -> Self {
        EsError {
            name: "".to_string(),
            message: err,
            stack: "".to_string(),
        }
    }
    pub fn get_message(&self) -> &str {
        self.message.as_str()
    }
    pub fn get_stack(&self) -> &str {
        self.stack.as_str()
    }
    pub fn get_name(&self) -> &str {
        self.name.as_str()
    }
}

impl std::fmt::Display for EsError {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), Error> {
        let e = format!("{}: {} at{}", self.name, self.message, self.stack);
        f.write_str(e.as_str())
    }
}

#[cfg(test)]
pub mod tests {
    use crate::esruntime::EsRuntime;
    use crate::esscript::EsScript;
    use crate::quickjs_utils::{errors, functions};
    use crate::quickjscontext::QuickJsContext;
    use crate::quickjsruntime::QuickJsRuntime;
    use crate::valueref::JSValueRef;
    use hirofa_utils::single_threaded_event_queue::SingleThreadedEventQueue;
    use libquickjs_sys as q;
    use std::ffi::CString;
    use std::sync::Arc;
    use std::thread;
    use std::time::Duration;

    struct RtWrapper {
        runtime: *mut q::JSRuntime,
        id: String,
    }

    impl RtWrapper {
        fn new() -> Self {
            //debug_assert!(SingleThreadedEventQueue::looks_like_eventqueue_thread());
            log::trace!("creating new QuickJsRuntime");
            let runtime = unsafe { q::JS_NewRuntime() };
            if runtime.is_null() {
                panic!("RuntimeCreationFailed");
            }
            let id = format!("q_{}", thread_id::get());
            Self { runtime, id }
        }
    }

    impl Drop for RtWrapper {
        fn drop(&mut self) {
            log::trace!("before JS_FreeRuntime");
            unsafe { q::JS_FreeRuntime(self.runtime) };
            log::trace!("after JS_FreeRuntime");
        }
    }

    //#[test]
    fn _test_err() {
        let eq = SingleThreadedEventQueue::new();
        let ok = eq.exe_task(|| {
            let q_js_rt = RtWrapper::new();
            //let q_js_rt = QuickJsRuntime::new();

            //let context = QuickJsContext::new("test".to_string(), &q_js_rt);
            //let context_raw = context.context;
            //let runtime = unsafe { q::JS_NewRuntime() };
            let runtime = q_js_rt.runtime;
            let context: *mut q::JSContext = unsafe { q::JS_NewContext(runtime) };

            let script = "(function err(){throw Error('Oh dear, stuff failed');});";
            let file_name = "err.js";
            let script_cstring = CString::new(script).unwrap();
            let filename_cstring = CString::new(file_name).unwrap();

            let value_raw = unsafe {
                q::JS_Eval(
                    context,
                    script_cstring.as_ptr(),
                    script.len() as _,
                    filename_cstring.as_ptr(),
                    q::JS_EVAL_TYPE_GLOBAL as i32,
                )
            };

            let f = JSValueRef::new(context, value_raw, false, true, "t1");
            //drop(script_cstring);
            //drop(script);
            //drop(filename_cstring);
            //drop(file_name);

            if f.is_exception() {
                let ex = unsafe { errors::get_exception(context).unwrap() };
                //panic!("failure: {}", ex);
            }

            let f_res = unsafe { functions::call_function(context, &f, vec![], None) };
            //drop(f);
            assert!(f_res.is_err());
            let err = format!("{}", f_res.err().unwrap());
            //let _l = err.len();
            //log::info!("err.len={}", l);
            //log::info!("err = {}", err);
            if !err.contains("Oh dear") {
                panic!("sdf, err was {}", err);
            }
            //panic!("sdf, err was {}", err);

            unsafe { q::JS_FreeContext(context) };

            true
        });
        std::thread::sleep(Duration::from_secs(1));
    }

    //#[test]
    fn _test_err2() {
        let rt: Arc<EsRuntime> = crate::esruntime::tests::TEST_ESRT.clone();
        rt.add_to_event_queue_sync(|q_js_rt| {
            //let context = QuickJsContext::new("test".to_string(), q_js_rt);
            let context = q_js_rt.get_main_context();
            let context_raw = context.context;

            let script = "(function err(){throw Error('Oh dear, stuff failed');});";
            let file_name = "err.js";

            let f = context
                .eval(EsScript::new(file_name, script))
                .ok()
                .expect("eval failed");

            if f.is_exception() {
                let ex = context.get_exception_ctx().unwrap();
                panic!("{}", ex);
            }

            let f_res = unsafe { functions::call_function(context_raw, &f, vec![], None) };
            drop(f);
            assert!(f_res.is_err());
            let err = format!("{}", f_res.err().unwrap());
            //let _l = err.len();
            //log::info!("err.len={}", l);
            //log::info!("err = {}", err);
            if !err.contains("Oh dear") {
                panic!("sdf, err was {}", err);
            }
            //panic!("sdf, err was {}", err);
        });
        std::thread::sleep(Duration::from_secs(1));
    }
}
