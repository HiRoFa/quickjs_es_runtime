use crate::quickjsruntime::QuickJsRuntime;

pub(crate) mod arrays;
pub(crate) mod atoms;
pub(crate) mod bigints;
pub(crate) mod functions;
pub(crate) mod modules;
pub(crate) mod objects;
pub(crate) mod primitives;
pub(crate) mod promises;
pub(crate) mod reflection;
pub(crate) mod typedarrays;
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
    JSValueRef::new_no_ref_ct_increment(q::JSValue {
        u: q::JSValueUnion { int32: 0 },
        tag: TAG_UNDEFINED,
    })
}

pub fn new_null() -> q::JSValue {
    q::JSValue {
        u: q::JSValueUnion { int32: 0 },
        tag: TAG_NULL,
    }
}

pub fn new_null_ref() -> JSValueRef {
    JSValueRef::new_no_ref_ct_increment(new_null())
}

pub fn get_global(q_js_rt: &QuickJsRuntime) -> JSValueRef {
    let global = unsafe { q::JS_GetGlobalObject(q_js_rt.context) };
    let mut global_ref = JSValueRef::new_no_ref_ct_increment(global);
    global_ref.label("global");
    global_ref
}
