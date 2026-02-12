//! low level contains utils for calling the quickjs api

use crate::quickjsruntimeadapter::QuickJsRuntimeAdapter;

#[cfg(feature = "bellard")]
pub mod class_ids {
    pub const JS_CLASS_OBJECT: u32 = 1;
    pub const JS_CLASS_ARRAY: u32 = 2;
    pub const JS_CLASS_ERROR: u32 = 3;
    pub const JS_CLASS_NUMBER: u32 = 4;
    pub const JS_CLASS_STRING: u32 = 5;
    pub const JS_CLASS_BOOLEAN: u32 = 6;
    pub const JS_CLASS_SYMBOL: u32 = 7;
    pub const JS_CLASS_ARGUMENTS: u32 = 8;
    pub const JS_CLASS_MAPPED_ARGUMENTS: u32 = 9;
    pub const JS_CLASS_DATE: u32 = 10;
    pub const JS_CLASS_MODULE_NS: u32 = 11;
    pub const JS_CLASS_C_FUNCTION: u32 = 12;
    pub const JS_CLASS_BYTECODE_FUNCTION: u32 = 13;
    pub const JS_CLASS_BOUND_FUNCTION: u32 = 14;
    pub const JS_CLASS_C_FUNCTION_DATA: u32 = 15;
    pub const JS_CLASS_GENERATOR_FUNCTION: u32 = 16;
    pub const JS_CLASS_FOR_IN_ITERATOR: u32 = 17;
    pub const JS_CLASS_REGEXP: u32 = 18;
    pub const JS_CLASS_ARRAY_BUFFER: u32 = 19;
    pub const JS_CLASS_SHARED_ARRAY_BUFFER: u32 = 20;
    pub const JS_CLASS_UINT8C_ARRAY: u32 = 21;
    pub const JS_CLASS_INT8_ARRAY: u32 = 22;
    pub const JS_CLASS_UINT8_ARRAY: u32 = 23;
    pub const JS_CLASS_INT16_ARRAY: u32 = 24;
    pub const JS_CLASS_UINT16_ARRAY: u32 = 25;
    pub const JS_CLASS_INT32_ARRAY: u32 = 26;
    pub const JS_CLASS_UINT32_ARRAY: u32 = 27;
    pub const JS_CLASS_BIG_INT64_ARRAY: u32 = 28;
    pub const JS_CLASS_BIG_UINT64_ARRAY: u32 = 29;
    pub const JS_CLASS_FLOAT16_ARRAY: u32 = 30;
    pub const JS_CLASS_FLOAT32_ARRAY: u32 = 31;
    pub const JS_CLASS_FLOAT64_ARRAY: u32 = 32;
    pub const JS_CLASS_DATAVIEW: u32 = 33;
    pub const JS_CLASS_BIG_INT: u32 = 34;
    pub const JS_CLASS_MAP: u32 = 35;
    pub const JS_CLASS_SET: u32 = 36;
    pub const JS_CLASS_WEAKMAP: u32 = 37;
    pub const JS_CLASS_WEAKSET: u32 = 38;
    pub const JS_CLASS_MAP_ITERATOR: u32 = 39;
    pub const JS_CLASS_SET_ITERATOR: u32 = 40;
    pub const JS_CLASS_ARRAY_ITERATOR: u32 = 41;
    pub const JS_CLASS_STRING_ITERATOR: u32 = 42;
    pub const JS_CLASS_REGEXP_STRING_ITERATOR: u32 = 43;
    pub const JS_CLASS_GENERATOR: u32 = 44;
    pub const JS_CLASS_PROXY: u32 = 45;
    pub const JS_CLASS_PROMISE: u32 = 46;
    pub const JS_CLASS_PROMISE_RESOLVE_FUNCTION: u32 = 47;
    pub const JS_CLASS_PROMISE_REJECT_FUNCTION: u32 = 48;
    pub const JS_CLASS_ASYNC_FUNCTION: u32 = 49;
    pub const JS_CLASS_ASYNC_FUNCTION_RESOLVE: u32 = 50;
    pub const JS_CLASS_ASYNC_FUNCTION_REJECT: u32 = 51;
    pub const JS_CLASS_ASYNC_FROM_SYNC_ITERATOR: u32 = 52;
    pub const JS_CLASS_ASYNC_GENERATOR_FUNCTION: u32 = 53;
    pub const JS_CLASS_ASYNC_GENERATOR: u32 = 54;
    pub const JS_CLASS_WEAK_REF: u32 = 55;
    pub const JS_CLASS_FINALIZATION_REGISTRY: u32 = 56;
    pub const JS_CLASS_INIT_COUNT: u32 = 57;
}
#[cfg(feature = "quickjs-ng")]
pub mod class_ids {
    pub const JS_CLASS_OBJECT: u32 = 1;
    pub const JS_CLASS_ARRAY: u32 = 2;
    pub const JS_CLASS_ERROR: u32 = 3;
    pub const JS_CLASS_NUMBER: u32 = 4;
    pub const JS_CLASS_STRING: u32 = 5;
    pub const JS_CLASS_BOOLEAN: u32 = 6;
    pub const JS_CLASS_SYMBOL: u32 = 7;
    pub const JS_CLASS_ARGUMENTS: u32 = 8;
    pub const JS_CLASS_MAPPED_ARGUMENTS: u32 = 9;
    pub const JS_CLASS_DATE: u32 = 10;
    pub const JS_CLASS_MODULE_NS: u32 = 11;
    pub const JS_CLASS_C_FUNCTION: u32 = 12;
    pub const JS_CLASS_BYTECODE_FUNCTION: u32 = 13;
    pub const JS_CLASS_BOUND_FUNCTION: u32 = 14;
    pub const JS_CLASS_C_FUNCTION_DATA: u32 = 15;
    pub const JS_CLASS_C_CLOSURE: u32 = 16;
    pub const JS_CLASS_GENERATOR_FUNCTION: u32 = 17;
    pub const JS_CLASS_FOR_IN_ITERATOR: u32 = 18;
    pub const JS_CLASS_REGEXP: u32 = 19;
    pub const JS_CLASS_ARRAY_BUFFER: u32 = 20;
    pub const JS_CLASS_SHARED_ARRAY_BUFFER: u32 = 21;
    pub const JS_CLASS_UINT8C_ARRAY: u32 = 22;
    pub const JS_CLASS_INT8_ARRAY: u32 = 23;
    pub const JS_CLASS_UINT8_ARRAY: u32 = 24;
    pub const JS_CLASS_INT16_ARRAY: u32 = 25;
    pub const JS_CLASS_UINT16_ARRAY: u32 = 26;
    pub const JS_CLASS_INT32_ARRAY: u32 = 27;
    pub const JS_CLASS_UINT32_ARRAY: u32 = 28;
    pub const JS_CLASS_BIG_INT64_ARRAY: u32 = 29;
    pub const JS_CLASS_BIG_UINT64_ARRAY: u32 = 30;
    pub const JS_CLASS_FLOAT16_ARRAY: u32 = 31;
    pub const JS_CLASS_FLOAT32_ARRAY: u32 = 32;
    pub const JS_CLASS_FLOAT64_ARRAY: u32 = 33;
    pub const JS_CLASS_DATAVIEW: u32 = 34;
    pub const JS_CLASS_BIG_INT: u32 = 35;
    pub const JS_CLASS_MAP: u32 = 36;
    pub const JS_CLASS_SET: u32 = 37;
    pub const JS_CLASS_WEAKMAP: u32 = 38;
    pub const JS_CLASS_WEAKSET: u32 = 39;
    pub const JS_CLASS_ITERATOR: u32 = 40;
    pub const JS_CLASS_ITERATOR_CONCAT: u32 = 41;
    pub const JS_CLASS_ITERATOR_HELPER: u32 = 42;
    pub const JS_CLASS_ITERATOR_WRAP: u32 = 43;
    pub const JS_CLASS_MAP_ITERATOR: u32 = 44;
    pub const JS_CLASS_SET_ITERATOR: u32 = 45;
    pub const JS_CLASS_ARRAY_ITERATOR: u32 = 46;
    pub const JS_CLASS_STRING_ITERATOR: u32 = 47;
    pub const JS_CLASS_REGEXP_STRING_ITERATOR: u32 = 48;
    pub const JS_CLASS_GENERATOR: u32 = 49;
    pub const JS_CLASS_PROXY: u32 = 50;
    pub const JS_CLASS_PROMISE: u32 = 51;
    pub const JS_CLASS_PROMISE_RESOLVE_FUNCTION: u32 = 52;
    pub const JS_CLASS_PROMISE_REJECT_FUNCTION: u32 = 53;
    pub const JS_CLASS_ASYNC_FUNCTION: u32 = 54;
    pub const JS_CLASS_ASYNC_FUNCTION_RESOLVE: u32 = 55;
    pub const JS_CLASS_ASYNC_FUNCTION_REJECT: u32 = 56;
    pub const JS_CLASS_ASYNC_FROM_SYNC_ITERATOR: u32 = 57;
    pub const JS_CLASS_ASYNC_GENERATOR_FUNCTION: u32 = 58;
    pub const JS_CLASS_ASYNC_GENERATOR: u32 = 59;
    pub const JS_CLASS_WEAK_REF: u32 = 60;
    pub const JS_CLASS_FINALIZATION_REGISTRY: u32 = 61;
    pub const JS_CLASS_DOM_EXCEPTION: u32 = 62;
    pub const JS_CLASS_CALL_SITE: u32 = 63;
    pub const JS_CLASS_INIT_COUNT: u32 = 64;
}

pub mod arrays;
pub mod atoms;
pub mod bigints;
pub mod compile;
pub mod dates;
pub mod errors;
pub mod functions;
pub mod interrupthandler;
pub mod iterators;
pub mod json;
pub mod maps;
pub mod modules;
pub mod objects;
pub mod primitives;
pub mod promises;
pub mod properties;
pub mod runtime;
pub mod sets;
pub mod typedarrays;

use crate::jsutils::JsError;
use crate::quickjs_utils::atoms::JSAtomRef;
use crate::quickjs_utils::objects::get_property;
use crate::quickjsrealmadapter::QuickJsRealmAdapter;
use crate::quickjsvalueadapter::{QuickJsValueAdapter, TAG_NULL, TAG_UNDEFINED};
use libquickjs_sys as q;

// todo
// runtime and context in thread_local here
// all function (where applicable) get an Option<QuickJSRuntime> which if None will be gotten from the thread_local
// every function which returns a q::JSValue will return a OwnedValueRef to ensure values are freed on drop

pub fn gc(q_js_rt: &QuickJsRuntimeAdapter) {
    log::trace!("GC called");
    unsafe { q::JS_RunGC(q_js_rt.runtime) }
    log::trace!("GC done");
}

pub fn new_undefined_ref() -> QuickJsValueAdapter {
    QuickJsValueAdapter::new_no_context(new_undefined(), "new_undefined_ref")
}

pub fn new_null() -> q::JSValue {
    q::JSValue {
        u: q::JSValueUnion { int32: 0 },
        tag: TAG_NULL,
    }
}

pub fn new_undefined() -> q::JSValue {
    q::JSValue {
        u: q::JSValueUnion { int32: 0 },
        tag: TAG_UNDEFINED,
    }
}

pub fn new_null_ref() -> QuickJsValueAdapter {
    QuickJsValueAdapter::new_no_context(new_null(), "null_ref")
}

/// get the current filename
pub fn get_script_or_module_name_q(ctx: &QuickJsRealmAdapter) -> Result<String, JsError> {
    unsafe { get_script_or_module_name(ctx.context) }
}

/// get the current filename
/// # Safety
/// ensure the QuickJsContext has not been dropped
pub unsafe fn get_script_or_module_name(context: *mut q::JSContext) -> Result<String, JsError> {
    for x in 0..100 {
        let atom = q::JS_GetScriptOrModuleName(context, x);
        let atom_ref = JSAtomRef::new(context, atom);
        let r = atoms::to_string(context, &atom_ref)?;
        if !r.is_empty() {
            return Ok(r);
        }
    }
    Ok("".to_string())
}

pub fn get_global_q(context: &QuickJsRealmAdapter) -> QuickJsValueAdapter {
    unsafe { get_global(context.context) }
}
/// # Safety
/// When passing a context pointer please make sure the corresponding QuickJsContext is still valid
pub unsafe fn get_global(context: *mut q::JSContext) -> QuickJsValueAdapter {
    let global = q::JS_GetGlobalObject(context);
    QuickJsValueAdapter::new(context, global, false, true, "global")
}
/// # Safety
/// When passing a context pointer please make sure the corresponding QuickJsContext is still valid
pub unsafe fn get_constructor(
    context: *mut q::JSContext,
    constructor_name: &str,
) -> Result<QuickJsValueAdapter, JsError> {
    let global_ref = get_global(context);

    let constructor_ref = get_property(context, &global_ref, constructor_name)?;

    if constructor_ref.is_null_or_undefined() {
        Err(JsError::new_string(format!(
            "not found: {constructor_name}"
        )))
    } else {
        Ok(constructor_ref)
    }
}
/// Calculate a runtimes memory usage
/// # Safety
/// runtime ref should be a valid existing runtime
pub unsafe fn get_memory_usage(runtime: *mut q::JSRuntime) -> q::JSMemoryUsage {
    let mut mu = q::JSMemoryUsage {
        malloc_size: 0,
        malloc_limit: 0,
        memory_used_size: 0,
        malloc_count: 0,
        memory_used_count: 0,
        atom_count: 0,
        atom_size: 0,
        str_count: 0,
        str_size: 0,
        obj_count: 0,
        obj_size: 0,
        prop_count: 0,
        prop_size: 0,
        shape_count: 0,
        shape_size: 0,
        js_func_count: 0,
        js_func_size: 0,
        js_func_code_size: 0,
        js_func_pc2line_count: 0,
        js_func_pc2line_size: 0,
        c_func_count: 0,
        array_count: 0,
        fast_array_count: 0,
        fast_array_elements: 0,
        binary_object_count: 0,
        binary_object_size: 0,
    };
    q::JS_ComputeMemoryUsage(runtime, &mut mu);

    mu
}

/// # Safety
/// When passing a context pointer please make sure the corresponding QuickJsContext is still valid
pub unsafe fn parse_args(
    context: *mut q::JSContext,
    argc: ::std::os::raw::c_int,
    argv: *mut q::JSValue,
) -> Vec<QuickJsValueAdapter> {
    let arg_slice = std::slice::from_raw_parts(argv, argc as usize);
    arg_slice
        .iter()
        .map(|raw| QuickJsValueAdapter::new(context, *raw, true, true, "quickjs_utils::parse_args"))
        .collect::<Vec<_>>()
}

#[cfg(test)]
pub mod tests {
    use crate::facades::tests::init_test_rt;
    use crate::jsutils::Script;
    use crate::quickjs_utils::{get_global_q, get_script_or_module_name_q};
    use crate::values::JsValueConvertable;

    #[test]
    fn test_global() {
        let rt = init_test_rt();
        rt.exe_rt_task_in_event_loop(|q_js_rt| {
            let q_ctx = q_js_rt.get_main_realm();

            #[cfg(feature = "bellard")]
            let ct = get_global_q(q_ctx).get_ref_count();
            for _ in 0..5 {
                let _global = get_global_q(q_ctx);
                #[cfg(feature = "bellard")]
                assert_eq!(_global.get_ref_count(), ct);
            }
        });
    }

    #[test]
    fn test_script_name() {
        let rt = init_test_rt();
        rt.set_function(&[], "testName", |q_ctx, _args| {
            let res = get_script_or_module_name_q(q_ctx)?.to_js_value_facade();
            Ok(res)
        })
        .ok()
        .expect("func set failed");
        let name_esvf = rt
            .eval_sync(
                None,
                Script::new("the_name.es", "(function(){return(testName());}())"),
            )
            .ok()
            .expect("script failed");
        assert_eq!(name_esvf.get_str(), "the_name.es");
        let name_esvf = rt
            .eval_sync(
                None,
                Script::new("https://githubstuff.org/tes.js", "(testName())"),
            )
            .ok()
            .expect("script failed");
        assert_eq!(name_esvf.get_str(), "https://githubstuff.org/tes.js");
    }
}
