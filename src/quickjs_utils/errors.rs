//! utils for getting and reporting exceptions

use crate::jsutils::JsError;
use crate::quickjs_utils::{objects, primitives};
use crate::quickjsrealmadapter::QuickJsRealmAdapter;
use crate::quickjsvalueadapter::{QuickJsValueAdapter, TAG_EXCEPTION};
use libquickjs_sys as q;

/// Get the last exception from the runtime, and if present, convert it to an JsError.
/// # Safety
/// When passing a context pointer please make sure the corresponding QuickJsContext is still valid
pub unsafe fn get_exception(context: *mut q::JSContext) -> Option<JsError> {
    log::trace!("get_exception");
    let exception_val = q::JS_GetException(context);
    log::trace!("get_exception / 2");
    let exception_ref =
        QuickJsValueAdapter::new(context, exception_val, false, true, "errors::get_exception");

    if exception_ref.is_null() {
        None
    } else {
        let err = if exception_ref.is_exception() {
            JsError::new_str("Could not get exception from runtime")
        } else if exception_ref.is_object() {
            error_to_js_error(context, &exception_ref)
        } else {
            JsError::new_str("no clue what happened")
        };
        Some(err)
    }
}

/// convert an instance of Error to JsError
/// # Safety
/// When passing a context pointer please make sure the corresponding QuickJsContext is still valid
pub unsafe fn error_to_js_error(
    context: *mut q::JSContext,
    exception_ref: &QuickJsValueAdapter,
) -> JsError {
    log::trace!("error_to_js_error");
    let name_ref = objects::get_property(context, exception_ref, "name")
        .ok()
        .unwrap();
    let name_string = primitives::to_string(context, &name_ref).ok().unwrap();
    let message_ref = objects::get_property(context, exception_ref, "message")
        .ok()
        .unwrap();
    let message_string = primitives::to_string(context, &message_ref).ok().unwrap();
    let stack_ref = objects::get_property(context, exception_ref, "stack")
        .ok()
        .unwrap();
    let mut stack_string = "".to_string();

    let stack2_ref = objects::get_property(context, exception_ref, "stack2")
        .ok()
        .unwrap();
    if stack2_ref.is_string() {
        stack_string.push_str(
            primitives::to_string(context, &stack2_ref)
                .ok()
                .unwrap()
                .as_str(),
        );
    }

    if stack_ref.is_string() {
        let stack_str = primitives::to_string(context, &stack_ref).ok().unwrap();
        #[cfg(feature = "typescript")]
        let stack_str = crate::typescript::unmap_stack_trace(stack_str.as_str());

        stack_string.push_str(stack_str.as_str());
    }

    JsError::new(name_string, message_string, stack_string)
}

/// Create a new Error object
/// # Safety
/// When passing a context pointer please make sure the corresponding QuickJsContext is still valid
pub unsafe fn new_error(
    context: *mut q::JSContext,
    name: &str,
    message: &str,
    stack: &str,
) -> Result<QuickJsValueAdapter, JsError> {
    let obj = q::JS_NewError(context);
    let obj_ref = QuickJsValueAdapter::new(
        context,
        obj,
        false,
        true,
        format!("new_error {name}").as_str(),
    );
    objects::set_property(
        context,
        &obj_ref,
        "message",
        &primitives::from_string(context, message)?,
    )?;
    objects::set_property(
        context,
        &obj_ref,
        "name",
        &primitives::from_string(context, name)?,
    )?;
    objects::set_property(
        context,
        &obj_ref,
        "stack2",
        &primitives::from_string(context, stack)?,
    )?;
    Ok(obj_ref)
}

/// See if a JSValueRef is an Error object
pub fn is_error_q(q_ctx: &QuickJsRealmAdapter, obj_ref: &QuickJsValueAdapter) -> bool {
    unsafe { is_error(q_ctx.context, obj_ref) }
}

/// See if a JSValueRef is an Error object
/// # Safety
/// When passing a context pointer please make sure the corresponding QuickJsContext is still valid
pub unsafe fn is_error(context: *mut q::JSContext, obj_ref: &QuickJsValueAdapter) -> bool {
    if obj_ref.is_object() {
        let res = q::JS_IsError(context, *obj_ref.borrow_value());
        res != 0
    } else {
        false
    }
}

/// Throw an error and get an Exception JSValue to return from native methods
/// # Safety
/// When passing a context pointer please make sure the corresponding QuickJsContext is still valid
pub unsafe fn throw(context: *mut q::JSContext, error: QuickJsValueAdapter) -> q::JSValue {
    assert!(is_error(context, &error));
    q::JS_Throw(context, error.clone_value_incr_rc());
    q::JSValue {
        u: q::JSValueUnion { int32: 0 },
        tag: TAG_EXCEPTION,
    }
}

#[cfg(test)]
pub mod tests {
    use crate::facades::tests::init_test_rt;
    use crate::jsutils::{JsError, Script};
    use crate::quickjs_utils::functions;
    use crate::values::{JsValueConvertable, JsValueFacade};
    //use log::LevelFilter;
    use std::time::Duration;

    #[test]
    fn test_ex_nat() {
        // check if stacktrace is preserved when invoking native methods

        let rt = init_test_rt();
        let res = rt.eval_sync(
            None,
            Script::new(
                "ex.js",
                "console.log('foo');\nconsole.log('bar');let a = __c_v__ * 7;",
            ),
        );
        let ex = res.expect_err("sciprt should have failed;");

        assert_eq!(ex.get_message(), "'__c_v__' is not defined");
    }

    #[test]
    fn test_ex0() {
        // check if stacktrace is preserved when invoking native methods

        let rt = init_test_rt();
        let res = rt.eval_sync(
            None,
            Script::new(
                "ex.js",
                "console.log('foo');\nconsole.log('bar');let a = __c_v__ * 7;",
            ),
        );
        let ex = res.expect_err("sciprt should have failed;");

        assert_eq!(ex.get_message(), "'__c_v__' is not defined");
    }

    #[test]
    fn test_ex1() {
        // check if stacktrace is preserved when invoking native methods

        let rt = init_test_rt();
        rt.set_function(&[], "test_consume", move |_realm, args| {
            // args[0] is a function i'll want to call
            let func_jsvf = &args[0];
            match func_jsvf {
                JsValueFacade::JsFunction { cached_function } => {
                    let _ = cached_function.invoke_function_sync(vec![12.to_js_value_facade()]);
                    Ok(0.to_js_value_facade())
                }
                _ => Err(JsError::new_str("poof")),
            }
        })
        .expect("could not set function");
        let s_res = rt.eval_sync(
            None,
            Script::new(
                "test_ex.es",
                "let consumer = function() {\n
        console.log('consuming');\n
        throw Error('oh dear stuff failed at line 3 in consumer');\n
        };\n
        console.log('calling consume from line 5');test_consume(consumer);\n
        console.log('should never reach line 7')",
            ),
        );
        if s_res.is_err() {
            let e = format!("script failed: {}", s_res.err().unwrap());
            log::error!("{}", e);
            //panic!("{}", e);
        }
        std::thread::sleep(Duration::from_secs(1));
    }

    #[test]
    fn test_ex2() {
        // check if stacktrace is preserved when invoking native methods

        //simple_logging::log_to_stderr(LevelFilter::Info);

        let rt = init_test_rt();
        rt.exe_rt_task_in_event_loop(|q_js_rt| {
            let q_ctx = q_js_rt.get_main_realm();

            q_ctx
                .eval(Script::new(
                    "test_ex2_pre.es",
                    "console.log('before ex test');",
                ))
                .expect("test_ex2_pre failed");
            {
                let func_ref1 = q_ctx
                    .eval(Script::new(
                        "test_ex2f1.es",
                        "(function(){\nconsole.log('running f1');});",
                    ))
                    .expect("script failed");
                assert!(functions::is_function_q(q_ctx, &func_ref1));
                let res = functions::call_function_q(q_ctx, &func_ref1, &[], None);
                match res {
                    Ok(_) => {}
                    Err(e) => {
                        log::error!("func1 failed: {}", e);
                    }
                }
            }
            // why the f does this fail with a stack overflow if i remove the block above?
            let func_ref2 = q_ctx
                .eval(Script::new(
                    "test_ex2.es",
                    r#"
                    const f = function(){
                        throw Error('poof');
                    };
                    f
                    "#,
                ))
                .expect("script failed");

            assert!(functions::is_function_q(q_ctx, &func_ref2));
            let res = functions::call_function_q(q_ctx, &func_ref2, &[], None);
            match res {
                Ok(_) => {}
                Err(e) => {
                    log::error!("func2 failed: {}", e);
                }
            }
        });

        let mjsvf = rt
            .eval_module_sync(
                None,
                Script::new(
                    "test_ex2.es",
                    r#"
                                throw Error('poof');
                                "#,
                ),
            )
            .expect("script compilation failed");
        match mjsvf {
            JsValueFacade::JsPromise { cached_promise } => {
                let pres = cached_promise
                    .get_promise_result_sync()
                    .expect("promise timed out");
                match pres {
                    Ok(m) => {
                        log::info!("prom resolved to {}", m.stringify())
                    }
                    Err(e) => {
                        log::info!("prom rejected to {}", e.stringify())
                    }
                }
            }
            _ => {
                panic!("not a prom")
            }
        }

        std::thread::sleep(Duration::from_secs(1));
    }
}
