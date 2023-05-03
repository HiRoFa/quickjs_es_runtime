//! Map utils, these methods can be used to manage Map objects from rust
//! see [MDN](https://developer.mozilla.org/en-US/docs/Web/JavaScript/Reference/Global_Objects/Map) for more on Maps

use crate::jsutils::JsError;
use crate::quickjs_utils::objects::{construct_object, is_instance_of_by_name};
use crate::quickjs_utils::{arrays, functions, get_constructor, iterators, objects, primitives};
use crate::quickjsrealmadapter::QuickJsRealmAdapter;
use crate::quickjsvalueadapter::QuickJsValueAdapter;
use libquickjs_sys as q;

/// create new instance of Map
/// # Example
/// ```rust
/// use quickjs_runtime::builder::QuickJsRuntimeBuilder;
/// use quickjs_runtime::quickjs_utils::maps::new_map_q;
/// use quickjs_runtime::quickjsvalueadapter::QuickJsValueAdapter;
///
/// let rt = QuickJsRuntimeBuilder::new().build();
/// rt.exe_rt_task_in_event_loop(|q_js_rt| {
///    let q_ctx = q_js_rt.get_main_context();
///    let my_map: QuickJsValueAdapter = new_map_q(q_ctx).ok().unwrap();
/// });
/// ```
pub fn new_map_q(q_ctx: &QuickJsRealmAdapter) -> Result<QuickJsValueAdapter, JsError> {
    unsafe { new_map(q_ctx.context) }
}

/// create new instance of Map
/// # Safety
/// please ensure the passed JSContext is still valid
pub unsafe fn new_map(ctx: *mut q::JSContext) -> Result<QuickJsValueAdapter, JsError> {
    let map_constructor = get_constructor(ctx, "Map")?;
    construct_object(ctx, &map_constructor, &[])
}

/// see if a JSValueRef is an instance of Map
pub fn is_map_q(q_ctx: &QuickJsRealmAdapter, obj: &QuickJsValueAdapter) -> Result<bool, JsError> {
    unsafe { is_map(q_ctx.context, obj) }
}

/// see if a JSValueRef is an instance of Map
/// # Safety
/// please ensure the passed JSContext is still valid
pub unsafe fn is_map(ctx: *mut q::JSContext, obj: &QuickJsValueAdapter) -> Result<bool, JsError> {
    is_instance_of_by_name(ctx, obj, "Map")
}

/// set a key/value pair in a Map
/// # Example
/// ```rust
/// use quickjs_runtime::builder::QuickJsRuntimeBuilder;
/// use quickjs_runtime::quickjs_utils::maps::{new_map_q, set_q};
/// use quickjs_runtime::quickjsvalueadapter::QuickJsValueAdapter;
/// use quickjs_runtime::quickjs_utils::primitives;
///
/// let rt = QuickJsRuntimeBuilder::new().build();
/// rt.exe_rt_task_in_event_loop(|q_js_rt| {
///    let q_ctx = q_js_rt.get_main_context();
///    let my_map: QuickJsValueAdapter = new_map_q(q_ctx).ok().unwrap();
///    let key = primitives::from_i32(12);
///    let value = primitives::from_i32(23);
///    set_q(q_ctx, &my_map, key, value).ok().unwrap();
/// });
/// ```
pub fn set_q(
    q_ctx: &QuickJsRealmAdapter,
    map: &QuickJsValueAdapter,
    key: QuickJsValueAdapter,
    val: QuickJsValueAdapter,
) -> Result<QuickJsValueAdapter, JsError> {
    unsafe { set(q_ctx.context, map, key, val) }
}

/// set a key/value pair in a Map
/// # Safety
/// please ensure the passed JSContext is still valid
pub unsafe fn set(
    ctx: *mut q::JSContext,
    map: &QuickJsValueAdapter,
    key: QuickJsValueAdapter,
    val: QuickJsValueAdapter,
) -> Result<QuickJsValueAdapter, JsError> {
    functions::invoke_member_function(ctx, map, "set", &[key, val])
}

/// get a value from a map by key
/// # Example
/// ```rust
/// use quickjs_runtime::builder::QuickJsRuntimeBuilder;
/// use quickjs_runtime::quickjs_utils::maps::{new_map_q, get_q, set_q};
/// use quickjs_runtime::quickjsvalueadapter::QuickJsValueAdapter;
/// use quickjs_runtime::quickjs_utils::primitives;
///
/// let rt = QuickJsRuntimeBuilder::new().build();
/// rt.exe_rt_task_in_event_loop(|q_js_rt| {
///    let q_ctx = q_js_rt.get_main_context();
///    let my_map: QuickJsValueAdapter = new_map_q(q_ctx).ok().unwrap();
///    let key = primitives::from_i32(12);
///    let value = primitives::from_i32(23);
///    set_q(q_ctx, &my_map, key.clone(), value).ok().unwrap();
///    let val_res = get_q(q_ctx, &my_map, key).ok().unwrap();
///    assert_eq!(primitives::to_i32(&val_res).ok().unwrap(), 23);
/// });
/// ```
pub fn get_q(
    q_ctx: &QuickJsRealmAdapter,
    map: &QuickJsValueAdapter,
    key: QuickJsValueAdapter,
) -> Result<QuickJsValueAdapter, JsError> {
    unsafe { get(q_ctx.context, map, key) }
}

/// get a value from a map by key
/// # Safety
/// please ensure the passed JSContext is still valid
pub unsafe fn get(
    ctx: *mut q::JSContext,
    map: &QuickJsValueAdapter,
    key: QuickJsValueAdapter,
) -> Result<QuickJsValueAdapter, JsError> {
    functions::invoke_member_function(ctx, map, "get", &[key])
}

/// delete a value from a map by key
/// # Example
/// ```rust
/// use quickjs_runtime::builder::QuickJsRuntimeBuilder;
/// use quickjs_runtime::quickjs_utils::maps::{new_map_q, set_q, delete_q};
/// use quickjs_runtime::quickjsvalueadapter::QuickJsValueAdapter;
/// use quickjs_runtime::quickjs_utils::primitives;
///
/// let rt = QuickJsRuntimeBuilder::new().build();
/// rt.exe_rt_task_in_event_loop(|q_js_rt| {
///    let q_ctx = q_js_rt.get_main_context();
///    let my_map: QuickJsValueAdapter = new_map_q(q_ctx).ok().unwrap();
///    let key = primitives::from_i32(12);
///    let value = primitives::from_i32(23);
///    set_q(q_ctx, &my_map, key.clone(), value).ok().unwrap();
///    delete_q(q_ctx, &my_map, key).ok().unwrap();
/// });
/// ```
pub fn delete_q(
    q_ctx: &QuickJsRealmAdapter,
    map: &QuickJsValueAdapter,
    key: QuickJsValueAdapter,
) -> Result<bool, JsError> {
    unsafe { delete(q_ctx.context, map, key) }
}

/// delete a value from a map by key
/// # Safety
/// please ensure the passed JSContext is still valid
pub unsafe fn delete(
    ctx: *mut q::JSContext,
    map: &QuickJsValueAdapter,
    key: QuickJsValueAdapter,
) -> Result<bool, JsError> {
    let res = functions::invoke_member_function(ctx, map, "delete", &[key])?;
    primitives::to_bool(&res)
}

/// check whether a Map has a value for a key
/// # Example
/// ```rust
/// use quickjs_runtime::builder::QuickJsRuntimeBuilder;
/// use quickjs_runtime::quickjs_utils::maps::{new_map_q, set_q, has_q};
/// use quickjs_runtime::quickjsvalueadapter::QuickJsValueAdapter;
/// use quickjs_runtime::quickjs_utils::primitives;
///
/// let rt = QuickJsRuntimeBuilder::new().build();
/// rt.exe_rt_task_in_event_loop(|q_js_rt| {
///    let q_ctx = q_js_rt.get_main_context();
///    let my_map: QuickJsValueAdapter = new_map_q(q_ctx).ok().unwrap();
///    let key = primitives::from_i32(12);
///    let value = primitives::from_i32(23);
///    set_q(q_ctx, &my_map, key.clone(), value).ok().unwrap();
///    let bln_has = has_q(q_ctx, &my_map, key).ok().unwrap();
///    assert!(bln_has);
/// });
/// ```
pub fn has_q(
    q_ctx: &QuickJsRealmAdapter,
    map: &QuickJsValueAdapter,
    key: QuickJsValueAdapter,
) -> Result<bool, JsError> {
    unsafe { has(q_ctx.context, map, key) }
}

/// check whether a Map has a value for a key
/// # Safety
/// please ensure the passed JSContext is still valid
pub unsafe fn has(
    ctx: *mut q::JSContext,
    map: &QuickJsValueAdapter,
    key: QuickJsValueAdapter,
) -> Result<bool, JsError> {
    let res = functions::invoke_member_function(ctx, map, "has", &[key])?;
    primitives::to_bool(&res)
}

/// get the number of entries in a map
/// # Example
/// ```rust
/// use quickjs_runtime::builder::QuickJsRuntimeBuilder;
/// use quickjs_runtime::quickjs_utils::maps::{new_map_q, set_q, size_q};
/// use quickjs_runtime::quickjsvalueadapter::QuickJsValueAdapter;
/// use quickjs_runtime::quickjs_utils::primitives;
///
/// let rt = QuickJsRuntimeBuilder::new().build();
/// rt.exe_rt_task_in_event_loop(|q_js_rt| {
///    let q_ctx = q_js_rt.get_main_context();
///    let my_map: QuickJsValueAdapter = new_map_q(q_ctx).ok().unwrap();
///    let key = primitives::from_i32(12);
///    let value = primitives::from_i32(23);
///    set_q(q_ctx, &my_map, key.clone(), value).ok().unwrap();
///    let i_size = size_q(q_ctx, &my_map).ok().unwrap();
///    assert_eq!(i_size, 1);
/// });
/// ```
pub fn size_q(q_ctx: &QuickJsRealmAdapter, map: &QuickJsValueAdapter) -> Result<i32, JsError> {
    unsafe { size(q_ctx.context, map) }
}

/// get the number of entries in a map
/// # Safety
/// please ensure the passed JSContext is still valid
pub unsafe fn size(ctx: *mut q::JSContext, map: &QuickJsValueAdapter) -> Result<i32, JsError> {
    let res = objects::get_property(ctx, map, "size")?;
    primitives::to_i32(&res)
}

/// remove all entries from a map
/// # Example
/// ```rust
/// use quickjs_runtime::builder::QuickJsRuntimeBuilder;
/// use quickjs_runtime::quickjs_utils::maps::{new_map_q, set_q, clear_q, size_q};
/// use quickjs_runtime::quickjsvalueadapter::QuickJsValueAdapter;
/// use quickjs_runtime::quickjs_utils::primitives;
///
/// let rt = QuickJsRuntimeBuilder::new().build();
/// rt.exe_rt_task_in_event_loop(|q_js_rt| {
///    let q_ctx = q_js_rt.get_main_context();
///    let my_map: QuickJsValueAdapter = new_map_q(q_ctx).ok().unwrap();
///    let key = primitives::from_i32(12);
///    let value = primitives::from_i32(23);
///    set_q(q_ctx, &my_map, key.clone(), value).ok().unwrap();
///    clear_q(q_ctx, &my_map).ok().unwrap();
///    let i_size = size_q(q_ctx, &my_map).ok().unwrap();
///    assert_eq!(i_size, 0);
/// });
/// ```
pub fn clear_q(q_ctx: &QuickJsRealmAdapter, map: &QuickJsValueAdapter) -> Result<(), JsError> {
    unsafe { clear(q_ctx.context, map) }
}

/// remove all entries from a map
/// # Safety
/// please ensure the passed JSContext is still valid
pub unsafe fn clear(ctx: *mut q::JSContext, map: &QuickJsValueAdapter) -> Result<(), JsError> {
    let _ = functions::invoke_member_function(ctx, map, "clear", &[])?;
    Ok(())
}

/// iterate over all keys of a map
/// # Example
/// ```rust
/// use quickjs_runtime::builder::QuickJsRuntimeBuilder;
/// use quickjs_runtime::quickjs_utils::maps::{new_map_q, set_q, keys_q};
/// use quickjs_runtime::quickjsvalueadapter::QuickJsValueAdapter;
/// use quickjs_runtime::quickjs_utils::primitives;
///
/// let rt = QuickJsRuntimeBuilder::new().build();
/// rt.exe_rt_task_in_event_loop(|q_js_rt| {
///    let q_ctx = q_js_rt.get_main_context();
///    let my_map: QuickJsValueAdapter = new_map_q(q_ctx).ok().unwrap();
///    let key = primitives::from_i32(12);
///    let value = primitives::from_i32(23);
///    set_q(q_ctx, &my_map, key, value).ok().unwrap();
///    let mapped_keys = keys_q(q_ctx, &my_map, |key| {Ok(123)}).ok().unwrap();
///    assert_eq!(mapped_keys.len(), 1);
/// });
/// ```
pub fn keys_q<C: Fn(QuickJsValueAdapter) -> Result<R, JsError>, R>(
    q_ctx: &QuickJsRealmAdapter,
    map: &QuickJsValueAdapter,
    consumer_producer: C,
) -> Result<Vec<R>, JsError> {
    unsafe { keys(q_ctx.context, map, consumer_producer) }
}

/// iterate over all keys of a map
/// # Safety
/// please ensure the passed JSContext is still valid
pub unsafe fn keys<C: Fn(QuickJsValueAdapter) -> Result<R, JsError>, R>(
    ctx: *mut q::JSContext,
    map: &QuickJsValueAdapter,
    consumer_producer: C,
) -> Result<Vec<R>, JsError> {
    let iter_ref = functions::invoke_member_function(ctx, map, "keys", &[])?;

    iterators::iterate(ctx, &iter_ref, consumer_producer)
}

/// iterate over all values of a map
/// # Example
/// ```rust
/// use quickjs_runtime::builder::QuickJsRuntimeBuilder;
/// use quickjs_runtime::quickjs_utils::maps::{new_map_q, set_q, values_q};
/// use quickjs_runtime::quickjsvalueadapter::QuickJsValueAdapter;
/// use quickjs_runtime::quickjs_utils::primitives;
///
/// let rt = QuickJsRuntimeBuilder::new().build();
/// rt.exe_rt_task_in_event_loop(|q_js_rt| {
///    let q_ctx = q_js_rt.get_main_context();
///    let my_map: QuickJsValueAdapter = new_map_q(q_ctx).ok().unwrap();
///    let key = primitives::from_i32(12);
///    let value = primitives::from_i32(23);
///    set_q(q_ctx, &my_map, key, value).ok().unwrap();
///    let mapped_values = values_q(q_ctx, &my_map, |value| {Ok(123)}).ok().unwrap();
///    assert_eq!(mapped_values.len(), 1);
/// });
/// ```
pub fn values_q<C: Fn(QuickJsValueAdapter) -> Result<R, JsError>, R>(
    q_ctx: &QuickJsRealmAdapter,
    map: &QuickJsValueAdapter,
    consumer_producer: C,
) -> Result<Vec<R>, JsError> {
    unsafe { values(q_ctx.context, map, consumer_producer) }
}

/// iterate over all values of a map
/// # Safety
/// please ensure the passed JSContext is still valid
pub unsafe fn values<C: Fn(QuickJsValueAdapter) -> Result<R, JsError>, R>(
    ctx: *mut q::JSContext,
    map: &QuickJsValueAdapter,
    consumer_producer: C,
) -> Result<Vec<R>, JsError> {
    let iter_ref = functions::invoke_member_function(ctx, map, "values", &[])?;

    iterators::iterate(ctx, &iter_ref, consumer_producer)
}

/// iterate over all entries of a map
/// # Example
/// ```rust
/// use quickjs_runtime::builder::QuickJsRuntimeBuilder;
/// use quickjs_runtime::quickjs_utils::maps::{new_map_q, set_q, entries_q};
/// use quickjs_runtime::quickjsvalueadapter::QuickJsValueAdapter;
/// use quickjs_runtime::quickjs_utils::primitives;
///
/// let rt = QuickJsRuntimeBuilder::new().build();
/// rt.exe_rt_task_in_event_loop(|q_js_rt| {
///    let q_ctx = q_js_rt.get_main_context();
///    let my_map: QuickJsValueAdapter = new_map_q(q_ctx).ok().unwrap();
///    let key = primitives::from_i32(12);
///    let value = primitives::from_i32(23);
///    set_q(q_ctx, &my_map, key, value).ok().unwrap();
///    let mapped_values = entries_q(q_ctx, &my_map, |key, value| {Ok(123)}).ok().unwrap();
///    assert_eq!(mapped_values.len(), 1);
/// });
/// ```
pub fn entries_q<C: Fn(QuickJsValueAdapter, QuickJsValueAdapter) -> Result<R, JsError>, R>(
    q_ctx: &QuickJsRealmAdapter,
    map: &QuickJsValueAdapter,
    consumer_producer: C,
) -> Result<Vec<R>, JsError> {
    unsafe { entries(q_ctx.context, map, consumer_producer) }
}

/// iterate over all entries of a map
/// # Safety
/// please ensure the passed JSContext is still valid
pub unsafe fn entries<C: Fn(QuickJsValueAdapter, QuickJsValueAdapter) -> Result<R, JsError>, R>(
    ctx: *mut q::JSContext,
    map: &QuickJsValueAdapter,
    consumer_producer: C,
) -> Result<Vec<R>, JsError> {
    let iter_ref = functions::invoke_member_function(ctx, map, "entries", &[])?;

    iterators::iterate(ctx, &iter_ref, |arr_ref| {
        let key = arrays::get_element(ctx, &arr_ref, 0)?;
        let value = arrays::get_element(ctx, &arr_ref, 1)?;
        consumer_producer(key, value)
    })
}
