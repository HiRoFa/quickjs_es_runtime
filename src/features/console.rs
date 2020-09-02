use crate::eserror::EsError;
use crate::quickjs_utils;
use crate::quickjs_utils::functions;
use crate::quickjs_utils::objects;
use crate::quickjs_utils::primitives;
use crate::quickjsruntime::{OwnedValueRef, QuickJsRuntime};
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
    argc: ::std::os::raw::c_int,
    argv: *mut q::JSValue,
) -> q::JSValue {
    //let mut argv_ref = OwnedValueRef::new(*argv);

    log::info!("console.log called, argc={}", argc,);

    let arg_slice = std::slice::from_raw_parts(argv, argc as usize);

    let mut args_vec: Vec<OwnedValueRef> = arg_slice
        .iter()
        .map(|raw| OwnedValueRef::new(*raw))
        .collect::<Vec<_>>();

    QuickJsRuntime::do_with(|q_js_rt| {
        for arg in &args_vec {
            log::trace!("arg tag = {}", arg.borrow_value().tag);
            if arg.is_string() {
                log::trace!(
                    "arg = {}",
                    primitives::to_string(q_js_rt, &arg).ok().expect("wtf1")
                );
            } else if arg.is_i32() {
                log::trace!("arg = {}", primitives::to_i32(&arg).ok().expect("wtf2"));
            }
        }
    });

    // prevent free
    while !args_vec.is_empty() {
        let mut arg_ref = args_vec.remove(0);
        let _ = arg_ref.consume_value();
    }

    quickjs_utils::new_null()
}

#[cfg(test)]
pub mod tests {
    use crate::esruntime::EsRuntime;
    use crate::esscript::EsScript;
    use std::sync::Arc;

    #[test]
    pub fn test_console() {
        let rt: Arc<EsRuntime> = crate::esruntime::tests::TEST_ESRT.clone();
        rt.eval_sync(EsScript::new(
            "test_console.es".to_string(),
            "console.log('one %s %s', 'two', 3)".to_string(),
        ))
        .ok()
        .expect("test_console.es failed");
    }
}
