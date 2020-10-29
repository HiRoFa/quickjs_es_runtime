use crate::eserror::EsError;
use crate::quickjs_utils;
use crate::quickjs_utils::reflection::Proxy;
use crate::quickjs_utils::{functions, parse_args};
use crate::quickjsruntime::QuickJsRuntime;
use libquickjs_sys as q;
use std::str::FromStr;

pub fn init(q_js_rt: &QuickJsRuntime) -> Result<(), EsError> {
    log::trace!("console::init");

    Proxy::new()
        .name("console")
        .static_native_method("log", Some(console_log))
        .static_native_method("trace", Some(console_trace))
        .static_native_method("info", Some(console_info))
        .static_native_method("warn", Some(console_warn))
        .static_native_method("error", Some(console_error))
        //.static_native_method("assert", Some(console_assert)) // todo
        .static_native_method("debug", Some(console_debug))
        .install(q_js_rt)
}

fn parse_field_value(field: &str, value: &str) -> String {
    // format ints
    // only support ,2 / .3 to declare the number of digits to display, e.g. $.3i turns 3 to 003

    // format floats
    // only support ,2 / .3 to declare the number of decimals to display, e.g. $.3f turns 3.1 to 3.100

    if field.eq(&"%.0f".to_string()) {
        return parse_field_value("%i", value);
    }

    if field.ends_with('d') || field.ends_with('i') {
        let mut i_val: String = value.to_string();

        // remove chars behind .
        if let Some(i) = i_val.find('.') {
            let _ = i_val.split_off(i);
        }

        if let Some(dot_in_field_idx) = field.find('.') {
            let mut m_field = field.to_string();
            // get part behind dot
            let mut num_decimals_str = m_field.split_off(dot_in_field_idx + 1);
            // remove d or i at end
            let _ = num_decimals_str.split_off(num_decimals_str.len() - 1);
            // see if we have a number
            if !num_decimals_str.is_empty() {
                let ct_res = usize::from_str(num_decimals_str.as_str());
                // check if we can parse the number to a usize
                if let Ok(ct) = ct_res {
                    // and if so, make i_val longer
                    while i_val.len() < ct {
                        i_val = format!("0{}", i_val);
                    }
                }
            }
        }

        return i_val;
    } else if field.ends_with('f') {
        let mut f_val = value.to_string();

        if let Some(dot_in_field_idx) = field.find('.') {
            let mut m_field = field.to_string();
            // get part behind dot
            let mut num_decimals_str = m_field.split_off(dot_in_field_idx + 1);
            // remove d or i at end
            let _ = num_decimals_str.split_off(num_decimals_str.len() - 1);
            // see if we have a number
            if !num_decimals_str.is_empty() {
                let ct_res = usize::from_str(num_decimals_str.as_str());
                // check if we can parse the number to a usize
                if let Ok(ct) = ct_res {
                    // and if so, make i_val longer
                    if ct > 0 {
                        if !f_val.contains('.') {
                            f_val.push('.');
                        }

                        let dot_idx = f_val.find('.').unwrap();

                        while f_val.len() - dot_idx <= ct {
                            f_val.push('0');
                        }
                        if f_val.len() - dot_idx > ct {
                            let _ = f_val.split_off(dot_idx + ct + 1);
                        }
                    }
                }
            }
        }

        return f_val;
    }
    value.to_string()
}

fn parse_line2(args: Vec<String>) -> String {
    if args.is_empty() {
        return "".to_string();
    }
    let message = &args[0];

    let mut output = String::new();
    let mut field_code = String::new();
    let mut in_field = false;

    let mut x = 1;

    for chr in message.chars() {
        if in_field {
            field_code.push(chr);
            if chr.eq(&'s') || chr.eq(&'d') || chr.eq(&'f') || chr.eq(&'o') || chr.eq(&'i') {
                // end field

                if x < args.len() {
                    output.push_str(parse_field_value(field_code.as_str(), &args[x]).as_str());
                    x += 1;
                }

                in_field = false;
                field_code = String::new();
            }
        } else if chr.eq(&'%') {
            in_field = true;
        } else {
            output.push(chr);
        }
    }

    output
}

unsafe fn parse_args_as_strings(argc: ::std::os::raw::c_int, argv: *mut q::JSValue) -> Vec<String> {
    let args_vec = parse_args(argc, argv);

    QuickJsRuntime::do_with(|q_js_rt| {
        args_vec
            .iter()
            .map(|arg| {
                functions::call_to_string(q_js_rt, arg)
                    .ok()
                    .expect("could not convert arg to string")
            })
            .collect::<Vec<String>>()
    })
}

unsafe extern "C" fn console_log(
    _ctx: *mut q::JSContext,
    _this_val: q::JSValue,
    argc: ::std::os::raw::c_int,
    argv: *mut q::JSValue,
) -> q::JSValue {
    log::trace!("> console.log");

    let args = parse_args_as_strings(argc, argv);

    log::info!("{}", parse_line2(args));

    quickjs_utils::new_null()
}

unsafe extern "C" fn console_trace(
    _ctx: *mut q::JSContext,
    _this_val: q::JSValue,
    argc: ::std::os::raw::c_int,
    argv: *mut q::JSValue,
) -> q::JSValue {
    log::trace!("> console.log");

    let args = parse_args_as_strings(argc, argv);

    log::trace!("{}", parse_line2(args));

    quickjs_utils::new_null()
}

unsafe extern "C" fn console_debug(
    _ctx: *mut q::JSContext,
    _this_val: q::JSValue,
    argc: ::std::os::raw::c_int,
    argv: *mut q::JSValue,
) -> q::JSValue {
    log::trace!("> console.log");

    let args = parse_args_as_strings(argc, argv);

    log::debug!("{}", parse_line2(args));

    quickjs_utils::new_null()
}

unsafe extern "C" fn console_info(
    _ctx: *mut q::JSContext,
    _this_val: q::JSValue,
    argc: ::std::os::raw::c_int,
    argv: *mut q::JSValue,
) -> q::JSValue {
    log::trace!("> console.log");

    let args = parse_args_as_strings(argc, argv);

    log::info!("{}", parse_line2(args));

    quickjs_utils::new_null()
}

unsafe extern "C" fn console_warn(
    _ctx: *mut q::JSContext,
    _this_val: q::JSValue,
    argc: ::std::os::raw::c_int,
    argv: *mut q::JSValue,
) -> q::JSValue {
    log::trace!("> console.log");

    let args = parse_args_as_strings(argc, argv);

    log::warn!("{}", parse_line2(args));

    quickjs_utils::new_null()
}

unsafe extern "C" fn console_error(
    _ctx: *mut q::JSContext,
    _this_val: q::JSValue,
    argc: ::std::os::raw::c_int,
    argv: *mut q::JSValue,
) -> q::JSValue {
    log::trace!("> console.log");

    let args = parse_args_as_strings(argc, argv);

    log::error!("{}", parse_line2(args));

    quickjs_utils::new_null()
}

#[cfg(test)]
pub mod tests {
    use crate::esruntime::EsRuntime;
    use crate::esscript::EsScript;
    use std::sync::Arc;

    #[test]
    pub fn test_console() {
        log::info!("> test_console");
        let rt: Arc<EsRuntime> = crate::esruntime::tests::TEST_ESRT.clone();
        rt.eval_sync(EsScript::new(
            "test_console.es",
            "console.log('one %s %s', 'two', 3)",
        ))
        .ok()
        .expect("test_console.es failed");
        log::info!("< test_console");
    }
}
