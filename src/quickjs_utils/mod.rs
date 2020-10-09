use crate::quickjsruntime::QuickJsRuntime;

pub mod arrays;
pub mod atoms;
pub mod bigints;
pub mod dates;
pub mod errors;
pub mod functions;
pub mod modules;
pub mod objects;
pub mod primitives;
pub mod promises;
pub mod reflection;
pub mod typedarrays;

use crate::eserror::EsError;
use crate::quickjs_utils::objects::get_property;
use crate::valueref::{JSValueRef, TAG_NULL, TAG_UNDEFINED};
use libquickjs_sys as q;

// todo
// runtime and context in thread_local here
// all function (where applicable) get an Option<QuickJSRuntime> which if None will be gotten from the thread_local
// every function which returns a q::JSValue will return a OwnedValueRef to ensure values are freed on drop

pub fn gc(q_js_rt: &QuickJsRuntime) {
    log::trace!("GC called");
    unsafe { q::JS_RunGC(q_js_rt.runtime) }
    log::trace!("GC done");
}

pub fn new_undefined_ref() -> JSValueRef {
    JSValueRef::new_no_ref_ct_increment(
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
    JSValueRef::new_no_ref_ct_increment(new_null(), "new_null_ref")
}

pub fn get_global(q_js_rt: &QuickJsRuntime) -> JSValueRef {
    let global = unsafe { q::JS_GetGlobalObject(q_js_rt.context) };
    let global_ref = JSValueRef::new(global, "global");
    global_ref
}

pub fn get_constructor(
    q_js_rt: &QuickJsRuntime,
    constructor_name: &str,
) -> Result<JSValueRef, EsError> {
    let global_ref = get_global(q_js_rt);

    let constructor_ref = get_property(q_js_rt, &global_ref, constructor_name)?;

    Ok(constructor_ref)
}
