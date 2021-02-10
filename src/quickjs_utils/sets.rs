//! Set utils, these methods can be used to manage Set objects from rust
//! see [MDN](https://developer.mozilla.org/en-US/docs/Web/JavaScript/Reference/Global_Objects/Sap) for more on Sets

//todo add, clear, delete, entries, has, values

use crate::eserror::EsError;
use crate::quickjs_utils::get_constructor;
use crate::quickjs_utils::objects::{construct_object, is_instance_of_by_name};
use crate::quickjscontext::QuickJsContext;
use crate::valueref::JSValueRef;
use libquickjs_sys as q;

/// create new instance of Set
/// # Example
/// ```rust
/// use quickjs_runtime::esruntimebuilder::EsRuntimeBuilder;
/// use quickjs_runtime::valueref::JSValueRef;
/// use quickjs_runtime::quickjs_utils::sets::new_set_q;
///
/// let rt = EsRuntimeBuilder::new().build();
/// rt.add_to_event_queue_sync(|q_js_rt| {
///    let q_ctx = q_js_rt.get_main_context();
///    let my_set: JSValueRef = new_set_q(q_ctx).ok().unwrap();
/// });
/// ```
pub fn new_set_q(q_ctx: &QuickJsContext) -> Result<JSValueRef, EsError> {
    unsafe { new_set(q_ctx.context) }
}

/// create new instance of Set
/// # Safety
/// please ensure the passed JSContext is still valid
pub unsafe fn new_set(ctx: *mut q::JSContext) -> Result<JSValueRef, EsError> {
    let map_constructor = get_constructor(ctx, "Set")?;
    construct_object(ctx, &map_constructor, vec![])
}

/// see if a JSValueRef is an instance of Set
pub fn is_set_q(q_ctx: &QuickJsContext, obj: &JSValueRef) -> Result<bool, EsError> {
    unsafe { is_set(q_ctx.context, obj) }
}

/// see if a JSValueRef is an instance of Set
/// # Safety
/// please ensure the passed JSContext is still valid
pub unsafe fn is_set(ctx: *mut q::JSContext, obj: &JSValueRef) -> Result<bool, EsError> {
    is_instance_of_by_name(ctx, obj, "Set")
}
