use crate::eserror::EsError;
use crate::quickjs_utils::{objects, primitives};
use crate::quickjsruntime::QuickJsRuntime;
use crate::valueref::{JSValueRef, TAG_EXCEPTION};
use libquickjs_sys as q;

/// Get the last exception from the runtime, and if present, convert it to an EsError.
pub fn get_exception(q_js_rt: &QuickJsRuntime) -> Option<EsError> {
    let exception_val = unsafe { q::JS_GetException(q_js_rt.context) };
    let mut exception_ref = JSValueRef::new(exception_val, false, true, "errors::get_exception");
    exception_ref.label("get_exception value obj");

    if exception_ref.is_null() {
        None
    } else {
        let err = if exception_ref.is_exception() {
            EsError::new_str("Could not get exception from runtime")
        } else if exception_ref.is_object() {
            let name_ref = objects::get_property(q_js_rt, &exception_ref, "name")
                .ok()
                .unwrap();
            let name_string = primitives::to_string(q_js_rt, &name_ref).ok().unwrap();
            let message_ref = objects::get_property(q_js_rt, &exception_ref, "message")
                .ok()
                .unwrap();
            let message_string = primitives::to_string(q_js_rt, &message_ref).ok().unwrap();
            let stack_ref = objects::get_property(q_js_rt, &exception_ref, "stack")
                .ok()
                .unwrap();
            let stack_string = primitives::to_string(q_js_rt, &stack_ref).ok().unwrap();

            EsError::new(name_string, message_string, stack_string)
        } else {
            EsError::new_str("no clue what happened")
        };
        Some(err)
    }
}

pub fn new_error(
    q_js_rt: &QuickJsRuntime,
    name: &str,
    message: &str,
    stack: &str,
) -> Result<JSValueRef, EsError> {
    let obj = unsafe { q::JS_NewError(q_js_rt.context) };
    let obj_ref = JSValueRef::new(obj, false, true, format!("new_error {}", name).as_str());
    objects::set_property(
        q_js_rt,
        &obj_ref,
        "message",
        primitives::from_string(q_js_rt, message)?,
    )?;
    objects::set_property(
        q_js_rt,
        &obj_ref,
        "name",
        primitives::from_string(q_js_rt, name)?,
    )?;
    objects::set_property(
        q_js_rt,
        &obj_ref,
        "stack",
        primitives::from_string(q_js_rt, stack)?,
    )?;
    Ok(obj_ref)
}

pub fn is_error(q_js_rt: &QuickJsRuntime, obj_ref: &JSValueRef) -> bool {
    if obj_ref.is_object() {
        let res = unsafe { q::JS_IsError(q_js_rt.context, *obj_ref.borrow_value()) };
        res != 0
    } else {
        false
    }
}

pub fn throw(q_js_rt: &QuickJsRuntime, error: JSValueRef) -> q::JSValue {
    assert!(is_error(q_js_rt, &error));
    unsafe { q::JS_Throw(q_js_rt.context, error.clone_value_incr_rc()) };
    q::JSValue {
        u: q::JSValueUnion { int32: 0 },
        tag: TAG_EXCEPTION,
    }
}

#[cfg(test)]
pub mod tests {
    use crate::esruntime::EsRuntime;
    use crate::esscript::EsScript;
    use crate::esvalue::EsValueConvertible;
    use crate::quickjs_utils::{functions, primitives};
    use std::sync::Arc;
    use std::time::Duration;

    #[test]
    fn test_ex() {
        // check if stacktrace is preserved when invoking native methods

        let rt: Arc<EsRuntime> = crate::esruntime::tests::TEST_ESRT.clone();
        rt.set_function(vec![], "test_consume", |args| {
            // args[0] is a function i'll want to call
            let func_esvf = &args[0];
            func_esvf.invoke_function_sync(vec![12.to_es_value_facade()])?;
            Ok(0.to_es_value_facade())
        })
        .ok()
        .expect("could not set function");
        let s_res = rt.eval_sync(EsScript::new(
            "test_ex.es",
            "let consumer = function() {\n
        console.log('consuming');\n
        throw Error('oh dear shit failed at line 3 in consumer');\n
        };\n
        console.log('calling consume from line 5');test_consume(consumer);\n
        console.log('should never reach line 7')",
        ));
        if s_res.is_err() {
            log::info!("script failed: {}", s_res.err().unwrap());
        }
        std::thread::sleep(Duration::from_secs(1));
    }

    #[test]
    fn test_ex2() {
        // check if stacktrace is preserved when invoking native methods

        let rt: Arc<EsRuntime> = crate::esruntime::tests::TEST_ESRT.clone();
        rt.add_to_event_queue_sync(|q_js_rt| {
            let func_ref = q_js_rt
                .eval(EsScript::new(
                    "test_ex2.es",
                    "(function t(){\nconsole.log('running f');\nthrow Error('poof');\n});",
                ))
                .ok()
                .expect("script failed");
            assert!(functions::is_function(q_js_rt, &func_ref));
            let res =
                functions::call_function(q_js_rt, &func_ref, vec![primitives::from_i32(12)], None);
            match res {
                Ok(_) => {}
                Err(e) => {
                    log::error!("func failed: {}", e);
                }
            }
        });

        std::thread::sleep(Duration::from_secs(1));
    }
}
