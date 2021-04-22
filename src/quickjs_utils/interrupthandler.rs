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
