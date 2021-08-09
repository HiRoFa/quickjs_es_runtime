//! Utils to compile script to bytecode and run script from bytecode

use crate::quickjsrealmadapter::QuickJsRealmAdapter;
use crate::quickjsruntimeadapter::make_cstring;
use crate::valueref::JSValueRef;
use hirofa_utils::js_utils::JsError;
use hirofa_utils::js_utils::Script;
use libquickjs_sys as q;
use std::os::raw::c_void;

/// compile a script, will result in a JSValueRef with tag JS_TAG_FUNCTION_BYTECODE or JS_TAG_MODULE.
///  It can be executed with run_compiled_function().
/// # Example
/// ```rust
/// use quickjs_runtime::builder::QuickJsRuntimeBuilder;
/// use hirofa_utils::js_utils::Script;
/// use quickjs_runtime::quickjs_utils::primitives;
/// use quickjs_runtime::quickjs_utils::compile::{compile, run_compiled_function};
/// let rt = QuickJsRuntimeBuilder::new().build();
/// rt.exe_rt_task_in_event_loop(|q_js_rt| {
///     unsafe {
///         let q_ctx = q_js_rt.get_main_context();
///         let func_res = compile(q_ctx.context, Script::new("test_func.es", "let a = 7; let b = 5; a * b;"));
///         let func = func_res.ok().expect("func compile failed");
///         let run_res = run_compiled_function(q_ctx.context, &func);
///         let res = run_res.ok().expect("run_compiled_function failed");
///         let i_res = primitives::to_i32(&res);
///         let i = i_res.ok().expect("could not convert to i32");
///         assert_eq!(i, 7*5);
///     }
/// });
/// ```
/// # Safety
/// When passing a context pointer please make sure the corresponding QuickJsContext is still valid
pub unsafe fn compile(context: *mut q::JSContext, script: Script) -> Result<JSValueRef, JsError> {
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
        false,
        true,
        format!("eval result of {}", script.get_path()).as_str(),
    );
    if ret.is_exception() {
        let ex_opt = QuickJsRealmAdapter::get_exception(context);
        if let Some(ex) = ex_opt {
            Err(ex)
        } else {
            Err(JsError::new_str(
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
) -> Result<JSValueRef, JsError> {
    assert!(compiled_func.is_compiled_function());
    let val = q::JS_EvalFunction(context, compiled_func.clone_value_incr_rc());
    let val_ref = JSValueRef::new(context, val, false, true, "run_compiled_function result");
    if val_ref.is_exception() {
        let ex_opt = QuickJsRealmAdapter::get_exception(context);
        if let Some(ex) = ex_opt {
            Err(ex)
        } else {
            Err(JsError::new_str(
                "run_compiled_function failed and could not get exception",
            ))
        }
    } else {
        Ok(val_ref)
    }
}

/// write a function to bytecode
/// # Example
/// ```rust
/// use quickjs_runtime::builder::QuickJsRuntimeBuilder;
/// use hirofa_utils::js_utils::Script;
/// use quickjs_runtime::quickjs_utils::primitives;
/// use quickjs_runtime::quickjs_utils::compile::{compile, run_compiled_function, to_bytecode, from_bytecode};
/// let rt = QuickJsRuntimeBuilder::new().build();
/// rt.exe_rt_task_in_event_loop(|q_js_rt| {
///     unsafe {
///     let q_ctx = q_js_rt.get_main_context();
///     let func_res = compile(q_ctx.context, Script::new("test_func.es", "let a = 7; let b = 5; a * b;"));
///     let func = func_res.ok().expect("func compile failed");
///     let bytecode: Vec<u8> = to_bytecode(q_ctx.context, &func);
///     drop(func);
///     assert!(!bytecode.is_empty());
///         let func2_res = from_bytecode(q_ctx.context, bytecode);
///         let func2 = func2_res.ok().expect("could not read bytecode");
///         let run_res = run_compiled_function(q_ctx.context, &func2);
///         let res = run_res.ok().expect("run_compiled_function failed");
///         let i_res = primitives::to_i32(&res);
///         let i = i_res.ok().expect("could not convert to i32");
///         assert_eq!(i, 7*5);
///     }
/// });
/// ```
/// # Safety
/// When passing a context pointer please make sure the corresponding QuickJsContext is still valid
pub unsafe fn to_bytecode(context: *mut q::JSContext, compiled_func: &JSValueRef) -> Vec<u8> {
    assert!(compiled_func.is_compiled_function());

    let mut len = 0;

    let slice_u8 = q::JS_WriteObject(
        context,
        &mut len,
        *compiled_func.borrow_value(),
        q::JS_WRITE_OBJ_BYTECODE as i32,
    );

    let slice = std::slice::from_raw_parts(slice_u8, len as usize);
    // it's a shame to copy the vec here but the alternative is to create a wrapping struct which free's the ptr on drop
    let ret = slice.to_vec();
    q::js_free(context, slice_u8 as *mut c_void);
    ret
}

/// read a compiled function from bytecode, see to_bytecode for an example
/// # Safety
/// When passing a context pointer please make sure the corresponding QuickJsContext is still valid
pub unsafe fn from_bytecode(
    context: *mut q::JSContext,
    bytecode: Vec<u8>,
) -> Result<JSValueRef, JsError> {
    assert!(!bytecode.is_empty());
    {
        let len = bytecode.len();

        let buf = bytecode.as_ptr();
        let raw = q::JS_ReadObject(context, buf, len as _, q::JS_READ_OBJ_BYTECODE as i32);

        let func_ref = JSValueRef::new(context, raw, false, true, "from_bytecode result");
        if func_ref.is_exception() {
            let ex_opt = QuickJsRealmAdapter::get_exception(context);
            if let Some(ex) = ex_opt {
                Err(ex)
            } else {
                Err(JsError::new_str(
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
    use crate::builder::QuickJsRuntimeBuilder;
    use crate::facades::tests::init_test_rt;
    use crate::quickjs_utils::compile::{
        compile, from_bytecode, run_compiled_function, to_bytecode,
    };
    use crate::quickjs_utils::primitives;
    use hirofa_utils::js_utils::Script;

    #[test]
    fn test_compile() {
        let rt = init_test_rt();

        rt.exe_rt_task_in_event_loop(|q_js_rt| {
            let q_ctx = q_js_rt.get_main_context();
            let func_res = unsafe {
                compile(
                    q_ctx.context,
                    Script::new(
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
        let rt = init_test_rt();
        rt.exe_rt_task_in_event_loop(|q_js_rt| unsafe {
            let q_ctx = q_js_rt.get_main_context();
            let func_res = compile(
                q_ctx.context,
                Script::new(
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

    #[test]
    fn test_bytecode_bad_compile() {
        let rt = QuickJsRuntimeBuilder::new().build();
        rt.exe_rt_task_in_event_loop(|q_js_rt| {
            let q_ctx = q_js_rt.get_main_context();

            let func_res = unsafe {
                compile(
                    q_ctx.context,
                    Script::new(
                        "test_func_fail.es",
                        "{the changes of me compil1ng a're slim to 0-0}",
                    ),
                )
            };
            func_res.err().expect("func compiled unexpectedly");
        })
    }

    #[test]
    fn test_bytecode_bad_run() {
        let rt = QuickJsRuntimeBuilder::new().build();
        rt.exe_rt_task_in_event_loop(|q_js_rt| unsafe {
            let q_ctx = q_js_rt.get_main_context();

            let func_res = compile(
                q_ctx.context,
                Script::new("test_func_runfail.es", "let abcdef = 1;"),
            );
            let func = func_res.ok().expect("func compile failed");
            assert_eq!(1, func.get_ref_count());

            let bytecode: Vec<u8> = to_bytecode(q_ctx.context, &func);

            assert_eq!(1, func.get_ref_count());

            drop(func);

            assert!(!bytecode.is_empty());

            let func2_res = from_bytecode(q_ctx.context, bytecode);
            let func2 = func2_res.ok().expect("could not read bytecode");
            //should fail the second time you run this because abcdef is already defined

            assert_eq!(1, func2.get_ref_count());

            let run_res1 = run_compiled_function(q_ctx.context, &func2)
                .ok()
                .expect("run 1 failed unexpectedly");
            drop(run_res1);

            assert_eq!(1, func2.get_ref_count());

            let _run_res2 = run_compiled_function(q_ctx.context, &func2)
                .err()
                .expect("run 2 succeeded unexpectedly");

            assert_eq!(1, func2.get_ref_count());
        });
    }
}
