use crate::quickjsruntime::QuickJsRuntime;
use libquickjs_sys as q;
use std::ffi::c_void;
use std::os::raw::c_int;

//

/// set an interrupt handler for the runtime
/// # Safety
/// be safe
pub unsafe fn set_interrupt_handler(runtime: *mut q::JSRuntime, handler: q::JSInterruptHandler) {
    q::JS_SetInterruptHandler(runtime, handler, std::ptr::null_mut());
}

pub(crate) fn init(q_js_rt: &QuickJsRuntime) {
    unsafe { set_interrupt_handler(q_js_rt.runtime, Some(interrupt_handler)) };
}

unsafe extern "C" fn interrupt_handler(_rt: *mut q::JSRuntime, _opaque: *mut c_void) -> c_int {
    QuickJsRuntime::do_with(|q_js_rt| {
        let handler = q_js_rt.interrupt_handler.as_ref().unwrap();
        if handler(q_js_rt) {
            1
        } else {
            0 // do not interrupt, return 1 to interrupt
        }
    })
}

#[cfg(test)]
pub mod tests {
    use crate::esruntimebuilder::EsRuntimeBuilder;
    use crate::esvalue::EsValueFacade;
    use hirofa_utils::js_utils::{JsError, Script};
    use std::cell::RefCell;
    use std::rc::Rc;
    use std::sync::{Arc, Mutex};

    #[test]
    fn test_interrupt_handler() {
        let called = Arc::new(Mutex::new(RefCell::new(false)));
        let called2 = called.clone();

        let rt = EsRuntimeBuilder::new()
            .set_interrupt_handler(move |qjs_rt| {
                println!("ihandler called");
                let lck = called2.lock().unwrap();
                *lck.borrow_mut() = true;
                false
            })
            .build();

        match rt.eval_sync(Script::new(
            "test_interrupt.es",
            "for (let x = 0; x < 10000; x++) {console.log('x' + x);}",
        )) {
            Ok(_) => {}
            Err(err) => {
                panic!("err: {}", err);
            }
        }

        let lck = called.lock().unwrap();
        assert!(*lck.borrow());
    }
}
