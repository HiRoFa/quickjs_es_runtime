use crate::facades::QuickJsRuntimeFacade;
use crate::jsutils::JsError;
use crate::quickjs_utils;
use crate::quickjs_utils::{functions, get_global_q, objects, parse_args};
use crate::quickjsruntimeadapter::QuickJsRuntimeAdapter;
use libquickjs_sys as q;

/// provides the setImmediate methods for the runtime
/// # Example
/// ```rust
/// use quickjs_runtime::builder::QuickJsRuntimeBuilder;
/// use quickjs_runtime::jsutils::Script;
/// use std::time::Duration;
/// let rt = QuickJsRuntimeBuilder::new().build();
/// rt.eval_sync(None, Script::new("test_immediate.es", "setImmediate(() => {console.log('immediate logging')});")).expect("script failed");
/// std::thread::sleep(Duration::from_secs(1));
/// ```
pub fn init(q_js_rt: &QuickJsRuntimeAdapter) -> Result<(), JsError> {
    log::trace!("setimmediate::init");

    q_js_rt.add_context_init_hook(|_q_js_rt, q_ctx| {
        let set_immediate_func =
            functions::new_native_function_q(q_ctx, "setImmediate", Some(set_immediate), 1, false)?;

        let global = get_global_q(q_ctx);

        objects::set_property2_q(q_ctx, &global, "setImmediate", &set_immediate_func, 0)?;
        Ok(())
    })?;
    Ok(())
}

unsafe extern "C" fn set_immediate(
    context: *mut q::JSContext,
    _this_val: q::JSValue,
    argc: ::std::os::raw::c_int,
    argv: *mut q::JSValue,
) -> q::JSValue {
    log::trace!("> set_immediate");

    let args = parse_args(context, argc, argv);

    QuickJsRuntimeAdapter::do_with(move |q_js_rt| {
        let q_ctx = q_js_rt.get_quickjs_context(context);
        if args.is_empty() {
            return q_ctx.report_ex("setImmediate requires at least one argument");
        }
        if !functions::is_function(context, &args[0]) {
            return q_ctx.report_ex("setImmediate requires a functions as first arg");
        }

        QuickJsRuntimeFacade::add_local_task_to_event_loop(move |_q_js_rt| {
            let func = &args[0];

            match functions::call_function(context, func, &args[1..], None) {
                Ok(_) => {}
                Err(e) => {
                    log::error!("setImmediate failed: {}", e);
                }
            };
        });

        quickjs_utils::new_null()
    })
}
