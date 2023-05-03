use crate::quickjsruntimeadapter::QuickJsRuntimeAdapter;
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

pub(crate) fn init(q_js_rt: &QuickJsRuntimeAdapter) {
    unsafe { set_interrupt_handler(q_js_rt.runtime, Some(interrupt_handler)) };
}

unsafe extern "C" fn interrupt_handler(_rt: *mut q::JSRuntime, _opaque: *mut c_void) -> c_int {
    QuickJsRuntimeAdapter::do_with(|q_js_rt| {
        let handler = q_js_rt.interrupt_handler.as_ref().unwrap();
        i32::from(handler(q_js_rt))
    })
}

#[cfg(test)]
pub mod tests {
    use crate::builder::QuickJsRuntimeBuilder;
    use crate::jsutils::Script;
    use crate::quickjs_utils::get_script_or_module_name_q;
    use backtrace::Backtrace;
    use log::LevelFilter;
    use std::cell::RefCell;
    use std::panic;
    use std::sync::{Arc, Mutex};

    #[test]
    fn test_interrupt_handler() {
        log::info!("interrupt_handler test");

        let called = Arc::new(Mutex::new(RefCell::new(false)));
        let called2 = called.clone();

        panic::set_hook(Box::new(|panic_info| {
            let backtrace = Backtrace::new();
            println!("thread panic occurred: {panic_info}\nbacktrace: {backtrace:?}");
            log::error!(
                "thread panic occurred: {}\nbacktrace: {:?}",
                panic_info,
                backtrace
            );
        }));

        simple_logging::log_to_file("esruntime.log", LevelFilter::max())
            .expect("could not init logger");

        let rt = QuickJsRuntimeBuilder::new()
            .set_interrupt_handler(move |qjs_rt| {
                log::debug!("interrupt_handler called / 1");
                let script_name = get_script_or_module_name_q(qjs_rt.get_main_context());
                match script_name {
                    Ok(script_name) => {
                        log::debug!("interrupt_handler called: {}", script_name);
                    }
                    Err(_) => {
                        log::debug!("interrupt_handler called");
                    }
                }
                let lck = called2.lock().unwrap();
                *lck.borrow_mut() = true;
                false
            })
            .build();

        match rt.eval_sync(
            None,
            Script::new(
                "test_interrupt.es",
                "for (let x = 0; x < 10000; x++) {console.log('x' + x);}",
            ),
        ) {
            Ok(_) => {}
            Err(err) => {
                panic!("err: {}", err);
            }
        }

        rt.create_context("newctx").expect("ctx crea failed");
        rt.exe_rt_task_in_event_loop(|q_js_rt| {
            let ctx = q_js_rt.get_context("newctx");
            match ctx.eval(Script::new(
                "test_interrupt.es",
                "for (let x = 0; x < 10000; x++) {console.log('x' + x);}",
            )) {
                Ok(_) => {}
                Err(err) => {
                    panic!("err: {}", err);
                }
            }
        });

        let lck = called.lock().unwrap();
        assert!(*lck.borrow());
    }
}
