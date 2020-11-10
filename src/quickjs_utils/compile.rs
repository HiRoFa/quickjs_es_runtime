use crate::eserror::EsError;
use crate::esscript::EsScript;
use crate::quickjsruntime::{make_cstring, QuickJsRuntime};
use crate::valueref::JSValueRef;
use libquickjs_sys as q;

/// compile a script, will result in a JSValueRef with tag JS_TAG_FUNCTION_BYTECODE or JS_TAG_MODULE.
///  It can be executed with run_compiled_function().
/// # Example
/// ```rust
/// use quickjs_es_runtime::esruntimebuilder::EsRuntimeBuilder;
/// use quickjs_es_runtime::esscript::EsScript;
/// use quickjs_es_runtime::quickjs_utils::primitives;
/// use quickjs_es_runtime::quickjs_utils::compile::{compile, run_compiled_function};
/// let rt = EsRuntimeBuilder::new().build();
/// rt.add_to_event_queue_sync(|q_js_rt| {
///     let func_res = compile(q_js_rt, EsScript::new("test_func.es", "let a = 7; let b = 5; a * b;"));
///     let func = func_res.ok().expect("func compile failed");
///     let run_res = run_compiled_function(q_js_rt, &func);
///     let res = run_res.ok().expect("run_compiled_function failed");
///     let i_res = primitives::to_i32(&res);
///     let i = i_res.ok().expect("could not convert to i32");
///     assert_eq!(i, 7*5);
/// });
/// ```
pub fn compile(q_js_rt: &QuickJsRuntime, script: EsScript) -> Result<JSValueRef, EsError> {
    let filename_c = make_cstring(script.get_path())?;
    let code_c = make_cstring(script.get_code())?;

    log::debug!("q_js_rt.compile file {}", script.get_path());

    let value_raw = unsafe {
        q::JS_Eval(
            q_js_rt.context,
            code_c.as_ptr(),
            script.get_code().len() as _,
            filename_c.as_ptr(),
            q::JS_EVAL_FLAG_COMPILE_ONLY as i32,
        )
    };

    log::trace!("after compile, checking error");

    // check for error
    let ret = JSValueRef::new(
        value_raw,
        true,
        true,
        format!("eval result of {}", script.get_path()).as_str(),
    );
    if ret.is_exception() {
        let ex_opt = q_js_rt.get_exception();
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
pub fn run_compiled_function(
    q_js_rt: &QuickJsRuntime,
    compiled_func: &JSValueRef,
) -> Result<JSValueRef, EsError> {
    assert!(compiled_func.is_compiled_function());
    let val = unsafe { q::JS_EvalFunction(q_js_rt.context, *compiled_func.borrow_value()) };
    let val_ref = JSValueRef::new(val, false, true, "run_compiled_function result");
    if val_ref.is_exception() {
        let ex_opt = q_js_rt.get_exception();
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
/// ```rust
/// use quickjs_es_runtime::esruntimebuilder::EsRuntimeBuilder;
/// use quickjs_es_runtime::esscript::EsScript;
/// use quickjs_es_runtime::quickjs_utils::primitives;
/// use quickjs_es_runtime::quickjs_utils::compile::{compile, run_compiled_function, to_bytecode, from_bytecode};
/// let rt = EsRuntimeBuilder::new().build();
/// rt.add_to_event_queue_sync(|q_js_rt| {
///     let func_res = compile(q_js_rt, EsScript::new("test_func.es", "let a = 7; let b = 5; a * b;"));
///     let func = func_res.ok().expect("func compile failed");
///     let bytecode: Vec<u8> = to_bytecode(q_js_rt, &func);
///     drop(func);
///     assert!(!bytecode.is_empty());
///     let func2_res = from_bytecode(q_js_rt, bytecode);
///     let func2 = func2_res.ok().expect("could not read bytecode");
///     let run_res = run_compiled_function(q_js_rt, &func2);
///     let res = run_res.ok().expect("run_compiled_function failed");
///     let i_res = primitives::to_i32(&res);
///     let i = i_res.ok().expect("could not convert to i32");
///     assert_eq!(i, 7*5);
/// });
/// ```
pub fn to_bytecode(q_js_rt: &QuickJsRuntime, compiled_func: &JSValueRef) -> Vec<u8> {
    assert!(compiled_func.is_compiled_function());

    let mut len: u64 = 0;
    let slice_u8 = unsafe {
        q::JS_WriteObject(
            q_js_rt.context,
            &mut len,
            *compiled_func.borrow_value(),
            q::JS_WRITE_OBJ_BYTECODE as i32,
        )
    };

    let slice = unsafe { std::slice::from_raw_parts(slice_u8, len as usize) };

    slice.to_vec()
}

/// read a compiled function from bytecode, see to_bytecode for an example
pub fn from_bytecode(q_js_rt: &QuickJsRuntime, bytecode: Vec<u8>) -> Result<JSValueRef, EsError> {
    assert!(!bytecode.is_empty());
    let len: u64 = bytecode.len() as u64;
    let buf = bytecode.as_ptr();
    let raw =
        unsafe { q::JS_ReadObject(q_js_rt.context, buf, len, q::JS_READ_OBJ_BYTECODE as i32) };

    let func_ref = JSValueRef::new(raw, true, true, "from_bytecode result");
    if func_ref.is_exception() {
        let ex_opt = q_js_rt.get_exception();
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
