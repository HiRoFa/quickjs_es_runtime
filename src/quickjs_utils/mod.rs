use crate::quickjsruntimeadapter::QuickJsRuntimeAdapter;

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
pub mod sets;
pub mod typedarrays;

use crate::quickjs_utils::atoms::JSAtomRef;
use crate::quickjs_utils::objects::get_property;
use crate::quickjsrealmadapter::QuickJsRealmAdapter;
use crate::valueref::{JSValueRef, TAG_NULL, TAG_UNDEFINED};
use hirofa_utils::js_utils::JsError;
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

pub fn new_undefined_ref() -> JSValueRef {
    JSValueRef::new_no_context(
        q::JSValue {
            u: q::JSValueUnion { int32: 0 },
            tag: TAG_UNDEFINED,
        },
        "new_undefined_ref",
    )
}

pub fn new_null() -> q::JSValue {
    q::JSValue {
        u: q::JSValueUnion { int32: 0 },
        tag: TAG_NULL,
    }
}

pub fn new_null_ref() -> JSValueRef {
    JSValueRef::new_no_context(new_null(), "null_ref")
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

pub fn get_global_q(context: &QuickJsRealmAdapter) -> JSValueRef {
    unsafe { get_global(context.context) }
}
/// # Safety
/// When passing a context pointer please make sure the corresponding QuickJsContext is still valid
pub unsafe fn get_global(context: *mut q::JSContext) -> JSValueRef {
    let global = q::JS_GetGlobalObject(context);
    JSValueRef::new(context, global, false, true, "global")
}
/// # Safety
/// When passing a context pointer please make sure the corresponding QuickJsContext is still valid
pub unsafe fn get_constructor(
    context: *mut q::JSContext,
    constructor_name: &str,
) -> Result<JSValueRef, JsError> {
    let global_ref = get_global(context);

    let constructor_ref = get_property(context, &global_ref, constructor_name)?;

    if constructor_ref.is_null_or_undefined() {
        Err(JsError::new_string(format!(
            "not found: {}",
            constructor_name
        )))
    } else {
        Ok(constructor_ref)
    }
}

/// # Safety
/// When passing a context pointer please make sure the corresponding QuickJsContext is still valid
pub unsafe fn parse_args(
    context: *mut q::JSContext,
    argc: ::std::os::raw::c_int,
    argv: *mut q::JSValue,
) -> Vec<JSValueRef> {
    let arg_slice = std::slice::from_raw_parts(argv, argc as usize);
    arg_slice
        .iter()
        .map(|raw| JSValueRef::new(context, *raw, true, true, "quickjs_utils::parse_args"))
        .collect::<Vec<_>>()
}

#[cfg(test)]
pub mod tests {
    use crate::esvalue::EsValueConvertible;
    use crate::facades::tests::init_test_rt;
    use crate::quickjs_utils::{get_global_q, get_script_or_module_name_q};
    use hirofa_utils::js_utils::Script;

    #[test]
    fn test_global() {
        let rt = init_test_rt();
        let _io = rt.exe_rt_task_in_event_loop(|q_js_rt| {
            let q_ctx = q_js_rt.get_main_context();

            let ct = get_global_q(q_ctx).get_ref_count();
            for _ in 0..5 {
                let global = get_global_q(q_ctx);
                assert_eq!(global.get_ref_count(), ct);
            }
        });
    }

    #[test]
    fn test_script_name() {
        let rt = init_test_rt();
        rt.set_function(vec![], "testName", |q_ctx, _args| {
            let res = get_script_or_module_name_q(q_ctx)?.to_es_value_facade();
            Ok(res)
        })
        .ok()
        .expect("func set failed");
        let name_esvf = rt
            .eval_sync(Script::new(
                "the_name.es",
                "(function(){return(testName());}())",
            ))
            .ok()
            .expect("script failed");
        assert_eq!(name_esvf.get_str(), "the_name.es");
        let name_esvf = rt
            .eval_sync(Script::new(
                "https://githubstuff.org/tes.js",
                "(testName())",
            ))
            .ok()
            .expect("script failed");
        assert_eq!(name_esvf.get_str(), "https://githubstuff.org/tes.js");
    }
}
