use crate::eserror::EsError;
use crate::quickjs_utils;
use crate::quickjs_utils::{functions, get_global, objects, parse_args};
use crate::quickjsruntime::QuickJsRuntime;
use libquickjs_sys as q;

/// provides the setImmediate methods for the runtime
/// # Example
/// ```rust
/// use quickjs_es_runtime::esruntimebuilder::EsRuntimeBuilder;
/// use quickjs_es_runtime::esscript::EsScript;
/// use std::time::Duration;
/// let rt = EsRuntimeBuilder::new().build();
/// rt.eval(EsScript::new("test_immediate.es", "setImmediate(() => {console.log('immediate logging')});"));
/// std::thread::sleep(Duration::from_secs(1));
/// ```

pub fn init(q_js_rt: &QuickJsRuntime) -> Result<(), EsError> {
    log::trace!("setimmediate::init");

    let set_immediate_func =
        functions::new_native_function(q_js_rt, "setImmediate", Some(set_immediate), 1, false)?;

    let global = get_global(q_js_rt);

    objects::set_property2(q_js_rt, &global, "setImmediate", set_immediate_func, 0)?;

    Ok(())
}

unsafe extern "C" fn set_immediate(
    _ctx: *mut q::JSContext,
    _this_val: q::JSValue,
    argc: ::std::os::raw::c_int,
    argv: *mut q::JSValue,
) -> q::JSValue {
    log::trace!("> set_immediate");

    let mut args = parse_args(argc, argv);

    QuickJsRuntime::do_with(move |q_js_rt| {
        if args.is_empty() {
            return q_js_rt.report_ex("setImmediate requires at least one argument");
        }
        if !functions::is_function(q_js_rt, &args[0]) {
            return q_js_rt.report_ex("setImmediate requires a functions as first arg");
        }

        if let Some(rt) = q_js_rt.get_rt_ref() {
            rt.inner.add_to_event_queue_from_worker(move |q_js_rt| {
                let func = args.remove(0);

                match functions::call_function(q_js_rt, &func, args, None) {
                    Ok(_) => {}
                    Err(e) => {
                        log::error!("setImmediate failed: {}", e);
                    }
                };
            });
        }
        quickjs_utils::new_null()
    })
}
