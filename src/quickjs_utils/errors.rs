//! utils for getting and reporting exceptions

use crate::quickjs_utils::{objects, primitives};
use crate::quickjscontext::QuickJsContext;
use crate::valueref::{JSValueRef, TAG_EXCEPTION};
use hirofa_utils::js_utils::JsError;
use libquickjs_sys as q;

/// Get the last exception from the runtime, and if present, convert it to an JsError.
/// # Safety
/// When passing a context pointer please make sure the corresponding QuickJsContext is still valid
pub unsafe fn get_exception(context: *mut q::JSContext) -> Option<JsError> {
    let exception_val = q::JS_GetException(context);
    let exception_ref =
        JSValueRef::new(context, exception_val, false, true, "errors::get_exception");

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
pub unsafe fn error_to_js_error(context: *mut q::JSContext, exception_ref: &JSValueRef) -> JsError {
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
    let stack_string;
    if stack_ref.is_string() {
        stack_string = primitives::to_string(context, &stack_ref).ok().unwrap();
    } else {
        stack_string = "".to_string();
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
) -> Result<JSValueRef, JsError> {
    let obj = q::JS_NewError(context);
    let obj_ref = JSValueRef::new(
        context,
        obj,
        false,
        true,
        format!("new_error {}", name).as_str(),
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
        "stack",
        &primitives::from_string(context, stack)?,
    )?;
    Ok(obj_ref)
}

/// See if a JSValueRef is an Error object
pub fn is_error_q(q_ctx: &QuickJsContext, obj_ref: &JSValueRef) -> bool {
    unsafe { is_error(q_ctx.context, obj_ref) }
}

/// See if a JSValueRef is an Error object
/// # Safety
/// When passing a context pointer please make sure the corresponding QuickJsContext is still valid
pub unsafe fn is_error(context: *mut q::JSContext, obj_ref: &JSValueRef) -> bool {
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
pub unsafe fn throw(context: *mut q::JSContext, error: JSValueRef) -> q::JSValue {
    assert!(is_error(context, &error));
    q::JS_Throw(context, error.clone_value_incr_rc());
    q::JSValue {
        u: q::JSValueUnion { int32: 0 },
        tag: TAG_EXCEPTION,
    }
}

#[cfg(test)]
pub mod tests {
    use crate::esruntime::tests::init_test_rt;
    use crate::esruntime::EsRuntime;
    use crate::esvalue::EsValueConvertible;
    use crate::quickjs_utils::functions;
    use hirofa_utils::js_utils::Script;
    use std::sync::Arc;
    use std::time::Duration;

    #[test]
    fn test_ex() {
        // check if stacktrace is preserved when invoking native methods

        let rt: Arc<EsRuntime> = init_test_rt();
        rt.set_function(vec![], "test_consume", |_q_ctx, args| {
            // args[0] is a function i'll want to call
            let func_esvf = &args[0];
            func_esvf.invoke_function_sync(vec![12.to_es_value_facade()])?;
            Ok(0.to_es_value_facade())
        })
        .ok()
        .expect("could not set function");
        let s_res = rt.eval_sync(Script::new(
            "test_ex.es",
            "let consumer = function() {\n
        console.log('consuming');\n
        throw Error('oh dear stuff failed at line 3 in consumer');\n
        };\n
        console.log('calling consume from line 5');test_consume(consumer);\n
        console.log('should never reach line 7')",
        ));
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

        let rt = init_test_rt();
        rt.exe_rt_task_in_event_loop(|q_js_rt| {
            let q_ctx = q_js_rt.get_main_context();

            q_ctx
                .eval(Script::new(
                    "test_ex2_pre.es",
                    "console.log('before ex test');",
                ))
                .ok()
                .expect("test_ex2_pre failed");
            {
                let func_ref1 = q_ctx
                    .eval(Script::new(
                        "test_ex2f1.es",
                        "(function(){\nconsole.log('running f1');});",
                    ))
                    .ok()
                    .expect("script failed");
                assert!(functions::is_function_q(q_ctx, &func_ref1));
                let res = functions::call_function_q(q_ctx, &func_ref1, vec![], None);
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
                    "(function(){\nconsole.log('running f2');\nthrow Error('poof');\n});",
                ))
                .ok()
                .expect("script failed");

            assert!(functions::is_function_q(q_ctx, &func_ref2));
            let res = functions::call_function_q(q_ctx, &func_ref2, vec![], None);
            match res {
                Ok(_) => {}
                Err(e) => {
                    log::error!("func2 failed: {}", e);
                }
            }
        });

        std::thread::sleep(Duration::from_secs(1));
    }
}
