use crate::eserror::EsError;
use crate::quickjs_utils;
use crate::quickjs_utils::functions;
use crate::quickjs_utils::objects;
use crate::quickjsruntime::QuickJsRuntime;
use libquickjs_sys as q;

pub fn init(q_js_rt: &QuickJsRuntime) -> Result<(), EsError> {
    log::trace!("console::init");

    let console_ref = objects::create_object(q_js_rt)?;

    let global_ref = quickjs_utils::get_global(q_js_rt);

    objects::set_property(q_js_rt, &global_ref, "console", console_ref)?;

    let log_func_ref = functions::new_native_function(q_js_rt, "log", Some(console_log), 1, false)?;

    let console_ref = objects::get_property(q_js_rt, &global_ref, "console")?;

    objects::set_property(q_js_rt, &console_ref, "log", log_func_ref)?;

    Ok(())
}

unsafe extern "C" fn console_log(
    _ctx: *mut q::JSContext,
    _this_val: q::JSValue,
    _argc: ::std::os::raw::c_int,
    _argv: *mut q::JSValue,
) -> q::JSValue {
    log::info!("console.log called");
    quickjs_utils::new_null()
}
