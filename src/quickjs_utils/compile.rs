use crate::eserror::EsError;
use crate::esscript::EsScript;
use crate::quickjscontext::QuickJsContext;
use crate::quickjsruntime::make_cstring;
use crate::valueref::JSValueRef;
use libquickjs_sys as q;

/// compile a script, will result in a JSValueRef with tag JS_TAG_FUNCTION_BYTECODE or JS_TAG_MODULE.
///  It can be executed with run_compiled_function().
/// # Example
/// ```dontrun
/// use quickjs_es_runtime::esruntimebuilder::EsRuntimeBuilder;
/// use quickjs_es_runtime::esscript::EsScript;
/// use quickjs_es_runtime::quickjs_utils::primitives;
/// use quickjs_es_runtime::quickjs_utils::compile::{compile, run_compiled_function};
/// let rt = EsRuntimeBuilder::new().build();
/// rt.add_to_event_queue_sync(|q_js_rt| {
///     let q_ctx = q_js_rt.get_main_context();
///     let func_res = compile(q_ctx.context, EsScript::new("test_func.es", "let a = 7; let b = 5; a * b;"));
///     let func = func_res.ok().expect("func compile failed");
///     let run_res = run_compiled_function(q_ctx.context, &func);
///     let res = run_res.ok().expect("run_compiled_function failed");
///     let i_res = primitives::to_i32(&res);
///     let i = i_res.ok().expect("could not convert to i32");
///     assert_eq!(i, 7*5);
/// });
/// ```
/// # Safety
/// When passing a context pointer please make sure the corresponding QuickJsContext is still valid
pub unsafe fn compile(context: *mut q::JSContext, script: EsScript) -> Result<JSValueRef, EsError> {
    let filename_c = make_cstring(script.get_path())?;
    let code_c = make_cstring(script.get_code())?;

    log::debug!("q_js_rt.compile file {}", script.get_path());

    let value_raw = q::JS_Eval(
        context,
        code_c.as_ptr(),
        script.get_code().len() as _,
        filename_c.as_ptr(),
        q::JS_EVAL_FLAG_COMPILE_ONLY as i32,
    );

    log::trace!("after compile, checking error");

    // check for error
    let ret = JSValueRef::new(
        context,
        value_raw,
        true,
        true,
        format!("eval result of {}", script.get_path()).as_str(),
    );
    if ret.is_exception() {
        let ex_opt = QuickJsContext::get_exception(context);
        if let Some(ex) = ex_opt {
            Err(ex)
        } else {
            Err(EsError::new_str(
                "compile failed and could not get exception",
            ))
        }
    } else {
        Ok(ret)
    }
}

/// run a compiled function, see compile for an example
/// # Safety
/// When passing a context pointer please make sure the corresponding QuickJsContext is still valid
pub unsafe fn run_compiled_function(
    context: *mut q::JSContext,
    compiled_func: &JSValueRef,
) -> Result<JSValueRef, EsError> {
    assert!(compiled_func.is_compiled_function());
    let val = q::JS_EvalFunction(context, *compiled_func.borrow_value());
    let val_ref = JSValueRef::new(context, val, false, true, "run_compiled_function result");
    if val_ref.is_exception() {
        let ex_opt = QuickJsContext::get_exception(context);
        if let Some(ex) = ex_opt {
            Err(ex)
        } else {
            Err(EsError::new_str(
                "run_compiled_function failed and could not get exception",
            ))
        }
    } else {
        Ok(val_ref)
    }
}

/// write a function to bytecode
/// # Example
/// ```dontrun
/// use quickjs_es_runtime::esruntimebuilder::EsRuntimeBuilder;
/// use quickjs_es_runtime::esscript::EsScript;
/// use quickjs_es_runtime::quickjs_utils::primitives;
/// use quickjs_es_runtime::quickjs_utils::compile::{compile, run_compiled_function, to_bytecode, from_bytecode};
/// let rt = EsRuntimeBuilder::new().build();
/// rt.add_to_event_queue_sync(|q_js_rt| {
///     let q_ctx = q_js_rt.get_main_context();
///     let func_res = compile(q_ctx.context, EsScript::new("test_func.es", "let a = 7; let b = 5; a * b;"));
///     let func = func_res.ok().expect("func compile failed");
///     let bytecode: Vec<u8> = to_bytecode(q_ctx.context, &func);
///     drop(func);
///     assert!(!bytecode.is_empty());
///     let func2_res = from_bytecode(q_ctx.context, bytecode);
///     let func2 = func2_res.ok().expect("could not read bytecode");
///     let run_res = run_compiled_function(q_ctx.context, &func2);
///     let res = run_res.ok().expect("run_compiled_function failed");
///     let i_res = primitives::to_i32(&res);
///     let i = i_res.ok().expect("could not convert to i32");
///     assert_eq!(i, 7*5);
/// });
/// ```
/// # Safety
/// When passing a context pointer please make sure the corresponding QuickJsContext is still valid
pub unsafe fn to_bytecode(context: *mut q::JSContext, compiled_func: &JSValueRef) -> Vec<u8> {
    assert!(compiled_func.is_compiled_function());

    #[cfg(target_pointer_width = "64")]
    let mut len: u64 = 0;
    #[cfg(target_pointer_width = "32")]
    let mut len: u32 = 0;

    let slice_u8 = q::JS_WriteObject(
        context,
        &mut len,
        *compiled_func.borrow_value(),
        q::JS_WRITE_OBJ_BYTECODE as i32,
    );

    let slice = std::slice::from_raw_parts(slice_u8, len as usize);

    slice.to_vec()
}

/// read a compiled function from bytecode, see to_bytecode for an example
/// # Safety
/// When passing a context pointer please make sure the corresponding QuickJsContext is still valid
pub unsafe fn from_bytecode(
    context: *mut q::JSContext,
    bytecode: Vec<u8>,
) -> Result<JSValueRef, EsError> {
    assert!(!bytecode.is_empty());
    {
        #[cfg(target_pointer_width = "64")]
        let len = bytecode.len() as u64;
        #[cfg(target_pointer_width = "32")]
        let len = bytecode.len() as u32;

        let buf = bytecode.as_ptr();
        let raw = q::JS_ReadObject(context, buf, len, q::JS_READ_OBJ_BYTECODE as i32);

        let func_ref = JSValueRef::new(context, raw, true, true, "from_bytecode result");
        if func_ref.is_exception() {
            let ex_opt = QuickJsContext::get_exception(context);
            if let Some(ex) = ex_opt {
                Err(ex)
            } else {
                Err(EsError::new_str(
                    "from_bytecode failed and could not get exception",
                ))
            }
        } else {
            Ok(func_ref)
        }
    }
}

#[cfg(test)]
pub mod tests {
    use crate::esruntime::EsRuntime;
    use crate::esscript::EsScript;
    use crate::quickjs_utils::compile::{
        compile, from_bytecode, run_compiled_function, to_bytecode,
    };
    use crate::quickjs_utils::primitives;
    use std::sync::Arc;

    #[test]
    fn test_compile() {
        let rt: Arc<EsRuntime> = crate::esruntime::tests::TEST_ESRT.clone();

        rt.add_to_event_queue_sync(|q_js_rt| {
            let q_ctx = q_js_rt.get_main_context();
            let func_res = unsafe {
                compile(
                    q_ctx.context,
                    EsScript::new(
                        "test_func.es",
                        "let a_tb3 = 7; let b_tb3 = 5; a_tb3 * b_tb3;",
                    ),
                )
            };
            let func = func_res.ok().expect("func compile failed");
            let bytecode: Vec<u8> = unsafe { to_bytecode(q_ctx.context, &func) };
            drop(func);
            assert!(!bytecode.is_empty());
            let func2_res = unsafe { from_bytecode(q_ctx.context, bytecode) };
            let func2 = func2_res.ok().expect("could not read bytecode");
            let run_res = unsafe { run_compiled_function(q_ctx.context, &func2) };
            match run_res {
                Ok(res) => {
                    let i_res = primitives::to_i32(&res);
                    let i = i_res.ok().expect("could not convert to i32");
                    assert_eq!(i, 7 * 5);
                }
                Err(e) => {
                    panic!("run failed1: {}", e);
                }
            }
        });
    }

    #[test]
    fn test_bytecode() {
        let rt: Arc<EsRuntime> = crate::esruntime::tests::TEST_ESRT.clone();
        rt.add_to_event_queue_sync(|q_js_rt| unsafe {
            let q_ctx = q_js_rt.get_main_context();
            let func_res = compile(
                q_ctx.context,
                EsScript::new(
                    "test_func.es",
                    "let a_tb4 = 7; let b_tb4 = 5; a_tb4 * b_tb4;",
                ),
            );
            let func = func_res.ok().expect("func compile failed");
            let bytecode: Vec<u8> = to_bytecode(q_ctx.context, &func);
            drop(func);
            assert!(!bytecode.is_empty());
            let func2_res = from_bytecode(q_ctx.context, bytecode);
            let func2 = func2_res.ok().expect("could not read bytecode");
            let run_res = run_compiled_function(q_ctx.context, &func2);

            match run_res {
                Ok(res) => {
                    let i_res = primitives::to_i32(&res);
                    let i = i_res.ok().expect("could not convert to i32");
                    assert_eq!(i, 7 * 5);
                }
                Err(e) => {
                    panic!("run failed: {}", e);
                }
            }
        });
    }
}
