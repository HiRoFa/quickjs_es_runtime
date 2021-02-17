//! utils for the iterator protocol

use crate::eserror::EsError;
use crate::quickjs_utils::{functions, objects, primitives};
use crate::valueref::JSValueRef;
use libquickjs_sys as q;

/// iterate over an object conforming to the [iterator](https://developer.mozilla.org/en-US/docs/Web/JavaScript/Reference/Iteration_protocols#the_iterator_protocol) protocol
/// # Safety
/// please ensure that the QuickjsContext corresponding to the passed JSContext is still valid
pub unsafe fn iterate<C: Fn(JSValueRef) -> Result<R, EsError>, R>(
    ctx: *mut q::JSContext,
    iterator_ref: &JSValueRef,
    consumer_producer: C,
) -> Result<Vec<R>, EsError> {
    let mut res = vec![];

    loop {
        let next_obj = functions::invoke_member_function(ctx, &iterator_ref, "next", vec![])?;
        if primitives::to_bool(&objects::get_property(ctx, &next_obj, "done")?)? {
            break;
        } else {
            let next_item = objects::get_property(ctx, &next_obj, "value")?;
            res.push(consumer_producer(next_item)?);
        }
    }

    Ok(res)
}
