//! the console feature enables the script to use various cansole.log variants
//! see also: [MDN](https://developer.mozilla.org/en-US/docs/Web/API/Console)
//! the following methods are available
//! * console.log()
//! * console.info()
//! * console.error()
//! * console.warning()
//! * console.trace()
//!
//! The methods use rust's log crate to output messages. e.g. console.info() uses the log::info!() macro
//! so the console messages should appear in the log you initialized from rust
//!
//! All methods accept a single message string and optional substitution values
//!
//! e.g.
//! ```javascript
//! console.log('Oh dear %s totaly failed %i times because of a %.4f variance in the space time continuum', 'some guy', 12, 2.46)
//! ```
//! will output 'Oh dear some guy totaly failed 12 times because of a 2.4600 variance in the space time continuum'
//!
//! The string substitution you can use are
//! * %o or %O Outputs a JavaScript object (serialized)
//! * %d or %i Outputs an integer. Number formatting is supported, for example  console.log("Foo %.2d", 1.1) will output the number as two significant figures with a leading 0: Foo 01
//! * %s Outputs a string (will attempt to call .toString() on objects, use %o to output a serialized JSON string)
//! * %f Outputs a floating-point value. Formatting is supported, for example  console.log("Foo %.2f", 1.1) will output the number to 2 decimal places: Foo 1.10
//! # Example
//! ```rust
//! use quickjs_runtime::builder::QuickJsRuntimeBuilder;
//! use hirofa_utils::js_utils::Script;
//! use log::LevelFilter;
//! simple_logging::log_to_file("console_test.log", LevelFilter::max())
//!             .ok()
//!             .expect("could not init logger");
//! let rt = QuickJsRuntimeBuilder::new().build();
//! rt.eval_sync(Script::new(
//! "console.es",
//! "console.log('the %s %s %s jumped over %i fences with a accuracy of %.2f', 'quick', 'brown', 'fox', 32, 0.512);"
//! ));
//! ```
//!
//! which will result in a log entry like
//! ```[00:00:00.012] (7f44e7d24700) INFO   the quick brown fox jumped over 32 fences with a accuracy of 0.51```

use crate::quickjs_utils;
use crate::quickjs_utils::functions::call_to_string;
use crate::quickjs_utils::{functions, json, parse_args, primitives};
use crate::quickjsrealmadapter::QuickJsRealmAdapter;
use crate::quickjsruntimeadapter::QuickJsRuntimeAdapter;
use crate::reflection::Proxy;
use crate::valueref::JSValueRef;
use hirofa_utils::js_utils::JsError;
use libquickjs_sys as q;
use log::LevelFilter;
use std::str::FromStr;

pub fn init(q_js_rt: &QuickJsRuntimeAdapter) -> Result<(), JsError> {
    q_js_rt.add_context_init_hook(|_q_js_rt, q_ctx| init_ctx(q_ctx))
}

pub(crate) fn init_ctx(q_ctx: &QuickJsRealmAdapter) -> Result<(), JsError> {
    Proxy::new()
        .name("console")
        .static_native_method("log", Some(console_log))
        .static_native_method("trace", Some(console_trace))
        .static_native_method("info", Some(console_info))
        .static_native_method("warn", Some(console_warn))
        .static_native_method("error", Some(console_error))
        //.static_native_method("assert", Some(console_assert)) // todo
        .static_native_method("debug", Some(console_debug))
        .install(q_ctx, true)
        .map(|_| {})
}

#[allow(clippy::or_fun_call)]
unsafe fn parse_field_value(ctx: *mut q::JSContext, field: &str, value: &JSValueRef) -> String {
    // format ints
    // only support ,2 / .3 to declare the number of digits to display, e.g. $.3i turns 3 to 003

    // format floats
    // only support ,2 / .3 to declare the number of decimals to display, e.g. $.3f turns 3.1 to 3.100

    if field.eq(&"%.0f".to_string()) {
        return parse_field_value(ctx, "%i", value);
    }

    if field.ends_with('d') || field.ends_with('i') {
        let mut i_val: String = call_to_string(ctx, value)
            .or::<String>(Ok("".to_string()))
            .unwrap();

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
        let mut f_val: String = call_to_string(ctx, value)
            .or::<String>(Ok("".to_string()))
            .unwrap();

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
            return f_val;
        } else if field.ends_with('o') || field.ends_with('O') {
            let json_str_res = json::stringify(ctx, value, None);
            let json = match json_str_res {
                Ok(json_str) => primitives::to_string(ctx, &json_str)
                    .or::<String>(Ok("".to_string()))
                    .unwrap(),
                Err(_e) => "".to_string(),
            };
            return json;
        }
    }
    call_to_string(ctx, value)
        .or::<String>(Ok("".to_string()))
        .unwrap()
}

#[allow(clippy::or_fun_call)]
unsafe fn parse_line(ctx: *mut q::JSContext, args: Vec<JSValueRef>) -> String {
    if args.is_empty() {
        return "".to_string();
    }

    let message = functions::call_to_string(ctx, &args[0])
        .or::<String>(Ok(String::new()))
        .unwrap();

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
                    output.push_str(parse_field_value(ctx, field_code.as_str(), &args[x]).as_str());
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

unsafe extern "C" fn console_log(
    ctx: *mut q::JSContext,
    _this_val: q::JSValue,
    argc: ::std::os::raw::c_int,
    argv: *mut q::JSValue,
) -> q::JSValue {
    if log::max_level() >= LevelFilter::Info {
        let args = parse_args(ctx, argc, argv);
        log::info!("{}", parse_line(ctx, args));
    }
    quickjs_utils::new_null()
}

unsafe extern "C" fn console_trace(
    ctx: *mut q::JSContext,
    _this_val: q::JSValue,
    argc: ::std::os::raw::c_int,
    argv: *mut q::JSValue,
) -> q::JSValue {
    if log::max_level() >= LevelFilter::Trace {
        let args = parse_args(ctx, argc, argv);
        log::trace!("{}", parse_line(ctx, args));
    }
    quickjs_utils::new_null()
}

unsafe extern "C" fn console_debug(
    ctx: *mut q::JSContext,
    _this_val: q::JSValue,
    argc: ::std::os::raw::c_int,
    argv: *mut q::JSValue,
) -> q::JSValue {
    if log::max_level() >= LevelFilter::Debug {
        let args = parse_args(ctx, argc, argv);
        log::debug!("{}", parse_line(ctx, args));
    }
    quickjs_utils::new_null()
}

unsafe extern "C" fn console_info(
    ctx: *mut q::JSContext,
    _this_val: q::JSValue,
    argc: ::std::os::raw::c_int,
    argv: *mut q::JSValue,
) -> q::JSValue {
    if log::max_level() >= LevelFilter::Info {
        let args = parse_args(ctx, argc, argv);
        log::info!("{}", parse_line(ctx, args));
    }
    quickjs_utils::new_null()
}

unsafe extern "C" fn console_warn(
    ctx: *mut q::JSContext,
    _this_val: q::JSValue,
    argc: ::std::os::raw::c_int,
    argv: *mut q::JSValue,
) -> q::JSValue {
    if log::max_level() >= LevelFilter::Warn {
        let args = parse_args(ctx, argc, argv);
        log::warn!("{}", parse_line(ctx, args));
    }
    quickjs_utils::new_null()
}

unsafe extern "C" fn console_error(
    ctx: *mut q::JSContext,
    _this_val: q::JSValue,
    argc: ::std::os::raw::c_int,
    argv: *mut q::JSValue,
) -> q::JSValue {
    if log::max_level() >= LevelFilter::Error {
        let args = parse_args(ctx, argc, argv);
        log::error!("{}", parse_line(ctx, args));
    }
    quickjs_utils::new_null()
}

#[cfg(test)]
pub mod tests {
    use crate::facades::tests::init_test_rt;
    use hirofa_utils::js_utils::Script;

    #[test]
    pub fn test_console() {
        log::info!("> test_console");
        let rt = init_test_rt();
        rt.eval_sync(Script::new(
            "test_console.es",
            "console.log('one %s %s', 'two', 3)",
        ))
        .ok()
        .expect("test_console.es failed");
        log::info!("< test_console");
    }
}
