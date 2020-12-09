use crate::eserror::EsError;
use crate::quickjs_utils;
use crate::quickjs_utils::{functions, get_global, objects, parse_args, primitives};
use crate::quickjsruntime::QuickJsRuntime;
use libquickjs_sys as q;
use std::time::Duration;

/// provides the setImmediate methods for the runtime
/// # Example
/// ```rust
/// use quickjs_es_runtime::esruntimebuilder::EsRuntimeBuilder;
/// use quickjs_es_runtime::esscript::EsScript;
/// use std::time::Duration;
/// let rt = EsRuntimeBuilder::new().build();
/// rt.eval(EsScript::new("test_timeout.es", "setTimeout(() => {console.log('timed logging')}, 1000);"));
/// std::thread::sleep(Duration::from_secs(2));
/// ```

pub fn init(q_js_rt: &QuickJsRuntime) -> Result<(), EsError> {
    log::trace!("set_timeout::init");

    q_js_rt.add_context_init_hook(|_q_js_rt, q_ctx| {
        let set_timeout_func =
            functions::new_native_function_q(q_ctx, "setTimeout", Some(set_timeout), 2, false)?;
        let set_interval_func =
            functions::new_native_function_q(q_ctx, "setInterval", Some(set_interval), 2, false)?;
        let clear_timeout_func =
            functions::new_native_function_q(q_ctx, "clearTimeout", Some(clear_timeout), 1, false)?;
        let clear_interval_func = functions::new_native_function_q(
            q_ctx,
            "clearInterval",
            Some(clear_interval),
            1,
            false,
        )?;

        let global = unsafe { get_global(q_ctx.context) };

        objects::set_property2_q(q_ctx, &global, "setTimeout", &set_timeout_func, 0)?;
        objects::set_property2_q(q_ctx, &global, "setInterval", &set_interval_func, 0)?;
        objects::set_property2_q(q_ctx, &global, "clearTimeout", &clear_timeout_func, 0)?;
        objects::set_property2_q(q_ctx, &global, "clearInterval", &clear_interval_func, 0)?;
        Ok(())
    })?;
    Ok(())
}

unsafe extern "C" fn set_timeout(
    context: *mut q::JSContext,
    _this_val: q::JSValue,
    argc: ::std::os::raw::c_int,
    argv: *mut q::JSValue,
) -> q::JSValue {
    log::trace!("> set_timeout");

    let mut args = parse_args(context, argc, argv);

    QuickJsRuntime::do_with(move |q_js_rt| {
        let q_ctx = q_js_rt.get_quickjs_context(context);
        if args.is_empty() {
            return q_ctx.report_ex("setTimeout requires at least one argument");
        }
        if !functions::is_function(context, &args[0]) {
            return q_ctx.report_ex("setTimeout requires a functions as first arg");
        }

        if args.len() >= 2 && !args[1].is_i32() && !args[1].is_f64() {
            return q_ctx.report_ex("setTimeout requires a number as second arg");
        }

        let delay_ms = if args.len() >= 2 {
            let delay_ref = args.remove(1);
            if delay_ref.is_i32() {
                primitives::to_i32(&delay_ref).ok().unwrap() as u64
            } else {
                primitives::to_f64(&delay_ref).ok().unwrap() as u64
            }
        } else {
            0
        };

        if let Some(rt) = q_js_rt.get_rt_ref() {
            let id = rt.inner.event_queue.schedule_task_from_worker(
                move || {
                    let mut args = args.clone();

                    let func = args.remove(0);

                    match functions::call_function(context, &func, args, None) {
                        Ok(_) => {}
                        Err(e) => {
                            log::error!("setTimeout func failed: {}", e);
                        }
                    };
                },
                None,
                Duration::from_millis(delay_ms),
            );
            log::trace!("set_timeout: {}", id);
            primitives::from_i32(id).clone_value_incr_rc()
        } else {
            quickjs_utils::new_null()
        }
    })
}

unsafe extern "C" fn set_interval(
    context: *mut q::JSContext,
    _this_val: q::JSValue,
    argc: ::std::os::raw::c_int,
    argv: *mut q::JSValue,
) -> q::JSValue {
    log::trace!("> set_interval");

    let mut args = parse_args(context, argc, argv);

    QuickJsRuntime::do_with(|q_js_rt| {
        let q_ctx = q_js_rt.get_quickjs_context(context);
        if args.is_empty() {
            return q_ctx.report_ex("setInterval requires at least one argument");
        }
        if !functions::is_function(context, &args[0]) {
            return q_ctx.report_ex("setInterval requires a functions as first arg");
        }

        if args.len() >= 2 && !args[1].is_i32() && !args[1].is_f64() {
            return q_ctx.report_ex("setInterval requires a number as second arg");
        }

        let delay_ms = if args.len() >= 2 {
            let delay_ref = args.remove(1);
            if delay_ref.is_i32() {
                primitives::to_i32(&delay_ref).ok().unwrap() as u64
            } else {
                primitives::to_f64(&delay_ref).ok().unwrap() as u64
            }
        } else {
            0
        };

        if let Some(rt) = q_js_rt.get_rt_ref() {
            let id = rt.inner.event_queue.schedule_task_from_worker(
                move || {
                    let mut args = args.clone();

                    let func = args.remove(0);

                    match functions::call_function(context, &func, args, None) {
                        Ok(_) => {}
                        Err(e) => {
                            log::error!("setInterval func failed: {}", e);
                        }
                    };
                },
                Some(Duration::from_millis(delay_ms)),
                Duration::from_millis(delay_ms),
            );
            log::trace!("set_interval: {}", id);
            primitives::from_i32(id).clone_value_incr_rc()
        } else {
            quickjs_utils::new_null()
        }
    })
}

unsafe extern "C" fn clear_interval(
    context: *mut q::JSContext,
    _this_val: q::JSValue,
    argc: ::std::os::raw::c_int,
    argv: *mut q::JSValue,
) -> q::JSValue {
    log::trace!("> clear_interval");

    let args = parse_args(context, argc, argv);
    QuickJsRuntime::do_with(|q_js_rt| {
        let q_ctx = q_js_rt.get_quickjs_context(context);
        if args.is_empty() {
            return q_ctx.report_ex("clearInterval requires at least one argument");
        }
        if !&args[0].is_i32() {
            return q_ctx.report_ex("clearInterval requires a number as first arg");
        }
        let id = primitives::to_i32(&args[0]).ok().unwrap();
        log::trace!("clear_interval: {}", id);
        if let Some(rt) = q_js_rt.get_rt_ref() {
            rt.inner.event_queue.remove_schedule_task_from_worker(id);
        };
        quickjs_utils::new_null()
    })
}

unsafe extern "C" fn clear_timeout(
    context: *mut q::JSContext,
    _this_val: q::JSValue,
    argc: ::std::os::raw::c_int,
    argv: *mut q::JSValue,
) -> q::JSValue {
    log::trace!("> clear_timeout");

    let args = parse_args(context, argc, argv);

    QuickJsRuntime::do_with(move |q_js_rt| {
        let q_ctx = q_js_rt.get_quickjs_context(context);
        if args.is_empty() {
            return q_ctx.report_ex("clearTimeout requires at least one argument");
        }
        if !&args[0].is_i32() {
            return q_ctx.report_ex("clearTimeout requires a number as first arg");
        }
        let id = primitives::to_i32(&args[0]).ok().unwrap();
        log::trace!("clear_timeout: {}", id);

        if let Some(rt) = q_js_rt.get_rt_ref() {
            rt.inner.event_queue.remove_schedule_task_from_worker(id);
        };

        quickjs_utils::new_null()
    })
}

#[cfg(test)]
pub mod tests {
    use crate::esruntime::EsRuntime;
    use crate::esscript::EsScript;
    use std::sync::Arc;
    use std::time::Duration;

    #[test]
    fn test_set_timeout() {
        let rt: Arc<EsRuntime> = crate::esruntime::tests::TEST_ESRT.clone();
        rt.eval(EsScript::new("test_set_timeout.es", "let t_id1 = setInterval((a, b) => {console.log('setInterval invoked with %s and %s', a, b);}, 500, 123, 456);"));
        rt.eval(EsScript::new("test_set_timeout.es", "let t_id2 = setTimeout((a, b) => {console.log('setTimeout invoked with %s and %s', a, b);}, 500, 123, 456);"));
        std::thread::sleep(Duration::from_secs(3));
        rt.eval(EsScript::new(
            "test_set_timeout2.es",
            "clearInterval(t_id1);",
        ));
        rt.eval(EsScript::new(
            "test_set_timeout2.es",
            "clearTimeout(t_id2);",
        ));
        std::thread::sleep(Duration::from_secs(2));
        rt.gc_sync();
    }
}
