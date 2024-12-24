//! Utils to compile script to bytecode and run script from bytecode

use crate::jsutils::JsError;
use crate::jsutils::Script;
use crate::quickjsrealmadapter::QuickJsRealmAdapter;
use crate::quickjsruntimeadapter::make_cstring;
use crate::quickjsvalueadapter::QuickJsValueAdapter;
use libquickjs_sys as q;
use std::os::raw::c_void;

/// compile a script, will result in a JSValueRef with tag JS_TAG_FUNCTION_BYTECODE or JS_TAG_MODULE.
///  It can be executed with run_compiled_function().
/// # Example
/// ```rust
/// use quickjs_runtime::builder::QuickJsRuntimeBuilder;
/// use quickjs_runtime::jsutils::Script;
/// use quickjs_runtime::quickjs_utils::primitives;
/// use quickjs_runtime::quickjs_utils::compile::{compile, run_compiled_function};
/// let rt = QuickJsRuntimeBuilder::new().build();
/// rt.exe_rt_task_in_event_loop(|q_js_rt| {
///     unsafe {
///         let q_ctx = q_js_rt.get_main_realm();
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
pub unsafe fn compile(
    context: *mut q::JSContext,
    script: Script,
) -> Result<QuickJsValueAdapter, JsError> {
    let filename_c = make_cstring(script.get_path())?;
    let code_str = script.get_runnable_code();
    let code_c = make_cstring(code_str)?;

    log::debug!("q_js_rt.compile file {}", script.get_path());

    let value_raw = q::JS_Eval(
        context,
        code_c.as_ptr(),
        code_str.len() as _,
        filename_c.as_ptr(),
        q::JS_EVAL_FLAG_COMPILE_ONLY as i32,
    );

    log::trace!("after compile, checking error");

    // check for error
    let ret = QuickJsValueAdapter::new(
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
    compiled_func: &QuickJsValueAdapter,
) -> Result<QuickJsValueAdapter, JsError> {
    assert!(compiled_func.is_compiled_function());
    let val = q::JS_EvalFunction(context, compiled_func.clone_value_incr_rc());
    let val_ref =
        QuickJsValueAdapter::new(context, val, false, true, "run_compiled_function result");
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
/// use quickjs_runtime::jsutils::Script;
/// use quickjs_runtime::quickjs_utils::primitives;
/// use quickjs_runtime::quickjs_utils::compile::{compile, run_compiled_function, to_bytecode, from_bytecode};
/// let rt = QuickJsRuntimeBuilder::new().build();
/// rt.exe_rt_task_in_event_loop(|q_js_rt| {
///     unsafe {
///     let q_ctx = q_js_rt.get_main_realm();
///     let func_res = compile(q_ctx.context, Script::new("test_func.es", "let a = 7; let b = 5; a * b;"));
///     let func = func_res.ok().expect("func compile failed");
///     let bytecode: Vec<u8> = to_bytecode(q_ctx.context, &func);
///     drop(func);
///     assert!(!bytecode.is_empty());
///         let func2_res = from_bytecode(q_ctx.context, &bytecode);
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
pub unsafe fn to_bytecode(
    context: *mut q::JSContext,
    compiled_func: &QuickJsValueAdapter,
) -> Vec<u8> {
    assert!(compiled_func.is_compiled_function() || compiled_func.is_module());

    let mut len = 0;

    let slice_u8 = q::JS_WriteObject(
        context,
        &mut len,
        *compiled_func.borrow_value(),
        q::JS_WRITE_OBJ_BYTECODE as i32,
    );

    let slice = std::slice::from_raw_parts(slice_u8, len as _);
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
    bytecode: &[u8],
) -> Result<QuickJsValueAdapter, JsError> {
    assert!(!bytecode.is_empty());
    {
        let len = bytecode.len();

        let buf = bytecode.as_ptr();
        let raw = q::JS_ReadObject(context, buf, len as _, q::JS_READ_OBJ_BYTECODE as i32);

        let func_ref = QuickJsValueAdapter::new(context, raw, false, true, "from_bytecode result");
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
    use crate::jsutils::modules::CompiledModuleLoader;
    use crate::jsutils::Script;
    use crate::quickjs_utils::compile::{
        compile, from_bytecode, run_compiled_function, to_bytecode,
    };
    use crate::quickjs_utils::modules::compile_module;
    use crate::quickjs_utils::primitives;
    use crate::quickjsrealmadapter::QuickJsRealmAdapter;
    use crate::values::JsValueFacade;
    //use backtrace::Backtrace;
    use futures::executor::block_on;
    use std::panic;
    use std::sync::Arc;

    #[test]
    fn test_compile() {
        let rt = init_test_rt();

        rt.exe_rt_task_in_event_loop(|q_js_rt| {
            let q_ctx = q_js_rt.get_main_realm();
            let func_res = unsafe {
                compile(
                    q_ctx.context,
                    Script::new(
                        "test_func.es",
                        "let a_tb3 = 7; let b_tb3 = 5; a_tb3 * b_tb3;",
                    ),
                )
            };
            let func = func_res.expect("func compile failed");
            let bytecode: Vec<u8> = unsafe { to_bytecode(q_ctx.context, &func) };
            drop(func);
            assert!(!bytecode.is_empty());
            let func2_res = unsafe { from_bytecode(q_ctx.context, &bytecode) };
            let func2 = func2_res.expect("could not read bytecode");
            let run_res = unsafe { run_compiled_function(q_ctx.context, &func2) };
            match run_res {
                Ok(res) => {
                    let i_res = primitives::to_i32(&res);
                    let i = i_res.expect("could not convert to i32");
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
            let q_ctx = q_js_rt.get_main_realm();
            let func_res = compile(
                q_ctx.context,
                Script::new(
                    "test_func.es",
                    "let a_tb4 = 7; let b_tb4 = 5; a_tb4 * b_tb4;",
                ),
            );
            let func = func_res.expect("func compile failed");
            let bytecode: Vec<u8> = to_bytecode(q_ctx.context, &func);
            drop(func);
            assert!(!bytecode.is_empty());
            let func2_res = from_bytecode(q_ctx.context, &bytecode);
            let func2 = func2_res.expect("could not read bytecode");
            let run_res = run_compiled_function(q_ctx.context, &func2);

            match run_res {
                Ok(res) => {
                    let i_res = primitives::to_i32(&res);
                    let i = i_res.expect("could not convert to i32");
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
            let q_ctx = q_js_rt.get_main_realm();

            let func_res = unsafe {
                compile(
                    q_ctx.context,
                    Script::new(
                        "test_func_fail.es",
                        "{the changes of me compil1ng a're slim to 0-0}",
                    ),
                )
            };
            func_res.expect_err("func compiled unexpectedly");
        })
    }

    #[test]
    fn test_bytecode_bad_run() {
        let rt = QuickJsRuntimeBuilder::new().build();
        rt.exe_rt_task_in_event_loop(|q_js_rt| unsafe {
            let q_ctx = q_js_rt.get_main_realm();

            let func_res = compile(
                q_ctx.context,
                Script::new("test_func_runfail.es", "let abcdef = 1;"),
            );
            let func = func_res.expect("func compile failed");
            #[cfg(feature = "bellard")]
            assert_eq!(1, func.get_ref_count());

            let bytecode: Vec<u8> = to_bytecode(q_ctx.context, &func);

            #[cfg(feature = "bellard")]
            assert_eq!(1, func.get_ref_count());

            drop(func);

            #[cfg(feature = "bellard")]
            assert!(!bytecode.is_empty());

            let func2_res = from_bytecode(q_ctx.context, &bytecode);
            let func2 = func2_res.expect("could not read bytecode");
            //should fail the second time you run this because abcdef is already defined

            #[cfg(feature = "bellard")]
            assert_eq!(1, func2.get_ref_count());

            let run_res1 =
                run_compiled_function(q_ctx.context, &func2).expect("run 1 failed unexpectedly");
            drop(run_res1);

            #[cfg(feature = "bellard")]
            assert_eq!(1, func2.get_ref_count());

            let _run_res2 = run_compiled_function(q_ctx.context, &func2)
                .expect_err("run 2 succeeded unexpectedly");

            #[cfg(feature = "bellard")]
            assert_eq!(1, func2.get_ref_count());
        });
    }

    lazy_static! {
        static ref COMPILED_BYTES: Arc<Vec<u8>> = init_bytes();
    }

    fn init_bytes() -> Arc<Vec<u8>> {
        // in order to init our bytes fgor our module we lazy init a rt
        let rt = QuickJsRuntimeBuilder::new().build();
        rt.loop_realm_sync(None, |_rt, realm| unsafe {
            let script = Script::new(
                "test_module.js",
                "export function someFunction(a, b){return a*b;};",
            );

            let module = compile_module(realm.context, script).expect("compile failed");

            Arc::new(to_bytecode(realm.context, &module))
        })
    }

    struct Cml {}
    impl CompiledModuleLoader for Cml {
        fn normalize_path(
            &self,
            _q_ctx: &QuickJsRealmAdapter,
            _ref_path: &str,
            path: &str,
        ) -> Option<String> {
            Some(path.to_string())
        }

        fn load_module(&self, _q_ctx: &QuickJsRealmAdapter, _absolute_path: &str) -> Arc<Vec<u8>> {
            COMPILED_BYTES.clone()
        }
    }

    #[test]
    fn test_bytecode_module() {
        /*panic::set_hook(Box::new(|panic_info| {
            let backtrace = Backtrace::new();
            println!("thread panic occurred: {panic_info}\nbacktrace: {backtrace:?}");
            log::error!(
                "thread panic occurred: {}\nbacktrace: {:?}",
                panic_info,
                backtrace
            );
        }));*/

        //simple_logging::log_to_file("quickjs_runtime.log", LevelFilter::max())
        //            .expect("could not init logger");

        let rt = QuickJsRuntimeBuilder::new()
            .compiled_module_loader(Cml {})
            .build();

        let test_script = Script::new(
            "test_bytecode_module.js",
            "import('testcompiledmodule').then((mod) => {return mod.someFunction(3, 5);})",
        );
        let res_fut = rt.eval(None, test_script);
        let res_prom = block_on(res_fut).expect("script failed");
        if let JsValueFacade::JsPromise { cached_promise } = res_prom {
            let prom_res_fut = cached_promise.get_promise_result();
            let prom_res = block_on(prom_res_fut)
                .expect("prom failed")
                .expect("prom was rejected");
            assert!(prom_res.is_i32());
            assert_eq!(prom_res.get_i32(), 15);
        } else {
            panic!("did not get a prom");
        }
    }
}
