use crate::quickjs_utils;
use crate::quickjs_utils::{functions, get_global, objects, parse_args, primitives};
use crate::quickjsruntime::QuickJsRuntimeAdapter;
use hirofa_utils::eventloop::EventLoop;
use hirofa_utils::js_utils::JsError;
use libquickjs_sys as q;
use std::time::Duration;

/// provides the setImmediate methods for the runtime
/// # Example
/// ```rust
/// use quickjs_runtime::builder::QuickjsRuntimeBuilder;
/// use hirofa_utils::js_utils::Script;
/// use std::time::Duration;
/// let rt = QuickjsRuntimeBuilder::new().build();
/// rt.eval(Script::new("test_timeout.es", "setTimeout(() => {console.log('timed logging')}, 1000);"));
/// std::thread::sleep(Duration::from_secs(2));
/// ```

pub fn init(q_js_rt: &QuickJsRuntimeAdapter) -> Result<(), JsError> {
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

    QuickJsRuntimeAdapter::do_with(move |q_js_rt| {
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

        let q_ctx_id = q_ctx.id.clone();

        let id = EventLoop::add_timeout(
            move || {
                QuickJsRuntimeAdapter::do_with(|q_js_rt| {
                    let mut args = args.clone();
                    let func = args.remove(0);
                    let q_ctx = q_js_rt.get_context(q_ctx_id.as_str());
                    match functions::call_function_q(q_ctx, &func, args, None) {
                        Ok(_) => {}
                        Err(e) => {
                            log::error!("setTimeout func failed: {}", e);
                        }
                    };
                    q_js_rt.run_pending_jobs_if_any();
                })
            },
            Duration::from_millis(delay_ms),
        );
        log::trace!("set_timeout: {}", id);
        primitives::from_i32(id).clone_value_incr_rc()
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

    QuickJsRuntimeAdapter::do_with(|q_js_rt| {
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

        let q_ctx_id = q_ctx.id.clone();

        let id = EventLoop::add_interval(
            move || {
                QuickJsRuntimeAdapter::do_with(|q_js_rt| {
                    let q_ctx = q_js_rt.get_context(q_ctx_id.as_str());
                    let mut args = args.clone();

                    let func = args.remove(0);

                    match functions::call_function_q(q_ctx, &func, args, None) {
                        Ok(_) => {}
                        Err(e) => {
                            log::error!("setInterval func failed: {}", e);
                        }
                    };

                    q_js_rt.run_pending_jobs_if_any();
                })
            },
            Duration::from_millis(delay_ms),
            Duration::from_millis(delay_ms),
        );
        log::trace!("set_interval: {}", id);
        primitives::from_i32(id).clone_value_incr_rc()
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
    QuickJsRuntimeAdapter::do_with(|q_js_rt| {
        let q_ctx = q_js_rt.get_quickjs_context(context);
        if args.is_empty() {
            return q_ctx.report_ex("clearInterval requires at least one argument");
        }
        if !&args[0].is_i32() {
            return q_ctx.report_ex("clearInterval requires a number as first arg");
        }
        let id = primitives::to_i32(&args[0]).ok().unwrap();
        log::trace!("clear_interval: {}", id);
        EventLoop::clear_interval(id);
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

    QuickJsRuntimeAdapter::do_with(move |q_js_rt| {
        let q_ctx = q_js_rt.get_quickjs_context(context);
        if args.is_empty() {
            return q_ctx.report_ex("clearTimeout requires at least one argument");
        }
        if !&args[0].is_i32() {
            return q_ctx.report_ex("clearTimeout requires a number as first arg");
        }
        let id = primitives::to_i32(&args[0]).ok().unwrap();
        log::trace!("clear_timeout: {}", id);

        EventLoop::clear_timeout(id);

        quickjs_utils::new_null()
    })
}

#[cfg(test)]
pub mod tests {
    use crate::facades::tests::init_test_rt;
    use crate::quickjs_utils::get_global_q;
    use crate::quickjs_utils::objects::get_property_q;
    use crate::quickjs_utils::primitives::to_i32;
    use hirofa_utils::js_utils::Script;
    use std::time::Duration;

    #[test]
    fn test_set_timeout_prom_res() {
        let rt = init_test_rt();
        let esvf = rt
            .eval_sync(Script::new(
                "test_set_timeout_prom_res.es",
                "new Promise((resolve, reject) => {\
                                setTimeout(() => {resolve(123);}, 1000);\
                            }).then((res) => {\
                                console.log(\"got %s\", res);\
                                return res + \"abc\";\
                            }).catch((ex) => {\
                                throw Error(\"\" + ex);\
                            });\
            ",
            ))
            .ok()
            .expect("script failed");
        assert!(esvf.is_promise());
        let res = esvf.get_promise_result_sync();
        assert!(res.is_ok());
        let res_str_esvf = res.ok().unwrap();
        let res_str = res_str_esvf.get_str();
        assert_eq!(res_str, "123abc");
        log::info!("done");
    }

    #[test]
    fn test_set_timeout_prom_res_nested() {
        let rt = init_test_rt();
        let esvf = rt
            .eval_sync(Script::new(
                "test_set_timeout_prom_res_nested.es",
                "new Promise((resolve, reject) => {\
                                setTimeout(() => {setTimeout(() => {resolve(123);}, 1000);}, 1000);\
                            }).then((res) => {\
                                console.log(\"got %s\", res);\
                                return res + \"abc\";\
                            }).catch((ex) => {\
                                throw Error(\"\" + ex);\
                            });\
            ",
            ))
            .ok()
            .expect("script failed");
        assert!(esvf.is_promise());
        let res = esvf.get_promise_result_sync();
        assert!(res.is_ok());
        let res_str_esvf = res.ok().unwrap();
        let res_str = res_str_esvf.get_str();
        assert_eq!(res_str, "123abc");
        log::info!("done");
    }

    #[test]
    fn test_set_timeout() {
        let rt = init_test_rt();

        rt.eval_sync(Script::new("test_set_interval.es", "let t_id1 = setInterval((a, b) => {console.log('setInterval invoked with %s and %s', a, b);}, 500, 123, 456);")).ok().expect("fail a");
        rt.eval_sync(Script::new("test_set_timeout.es", "let t_id2 = setTimeout((a, b) => {console.log('setTimeout1 invoked with %s and %s', a, b);}, 500, 123, 456);")).ok().expect("fail b");
        rt.eval_sync(Script::new("test_set_timeout.es", "let t_id3 = setTimeout((a, b) => {console.log('setTimeout2 invoked with %s and %s', a, b);}, 600, 123, 456);")).ok().expect("fail b");
        rt.eval_sync(Script::new("test_set_timeout.es", "let t_id4 = setTimeout((a, b) => {console.log('setTimeout3 invoked with %s and %s', a, b);}, 900, 123, 456);")).ok().expect("fail b");
        std::thread::sleep(Duration::from_secs(3));
        rt.eval_sync(Script::new(
            "test_clearInterval.es",
            "clearInterval(t_id1);",
        ))
        .ok()
        .expect("fail c");
        rt.eval_sync(Script::new("test_clearTimeout2.es", "clearTimeout(t_id2);"))
            .ok()
            .expect("fail d");

        rt.eval_sync(Script::new("test_set_timeout2.es", "this.__ti_num__ = 0;"))
            .ok()
            .expect("fail qewr");

        rt.eval_sync(Script::new("test_set_timeout2.es", "this.__it_num__ = 0;"))
            .ok()
            .expect("fail qewr");

        rt.eval_sync(Script::new(
            "test_set_timeout3.es",
            "setTimeout(() => {console.log('seto1');this.__ti_num__++;}, 455);",
        ))
        .ok()
        .expect("fail a1");
        rt.eval_sync(Script::new(
            "test_set_timeout3.es",
            "setTimeout(() => {console.log('seto2');this.__ti_num__++;}, 366);",
        ))
        .ok()
        .expect("fail a2");
        rt.eval_sync(Script::new(
            "test_set_timeout3.es",
            "setTimeout(() => {console.log('seto3');this.__ti_num__++;}, 1001);",
        ))
        .ok()
        .expect("fail a3");
        rt.eval_sync(Script::new(
            "test_set_timeout3.es",
            "setTimeout(() => {console.log('seto4');this.__ti_num__++;}, 2002);",
        ))
        .ok()
        .expect("fail a4");

        rt.eval_sync(Script::new(
            "test_set_interval.es",
            "setInterval(() => {this.__it_num__++;}, 1600);",
        ))
        .ok()
        .expect("fail a");
        rt.eval_sync(Script::new(
            "test_set_interval.es",
            "setInterval(() => {this.__it_num__++;}, 2500);",
        ))
        .ok()
        .expect("fail a");

        std::thread::sleep(Duration::from_secs(6));

        let i = rt.exe_rt_task_in_event_loop(|q_js_rt| {
            let q_ctx = q_js_rt.get_main_context();
            let global = get_global_q(q_ctx);
            let ti_num = get_property_q(q_ctx, &global, "__ti_num__")
                .ok()
                .expect("could not get ti num prop from global");
            let it_num = get_property_q(q_ctx, &global, "__it_num__")
                .ok()
                .expect("could not get it num prop from global");

            (
                to_i32(&ti_num)
                    .ok()
                    .expect("could not convert ti num to num"),
                to_i32(&it_num)
                    .ok()
                    .expect("could not convert it num to num"),
            )
        });
        assert_eq!(i.1, 5);
        assert_eq!(i.0, 4);

        rt.gc_sync();
    }
}
