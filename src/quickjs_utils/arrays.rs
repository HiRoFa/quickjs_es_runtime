use crate::quickjscontext::QuickJsContext;
use crate::valueref::JSValueRef;
use hirofa_utils::js_utils::JsError;
use libquickjs_sys as q;

/// Check whether an object is an array
/// # Example
/// ```rust
/// use quickjs_runtime::esruntimebuilder::EsRuntimeBuilder;
/// use hirofa_utils::js_utils::Script;
/// use quickjs_runtime::quickjs_utils::arrays;
///
/// let rt = EsRuntimeBuilder::new().build();
/// rt.exe_rt_task_in_event_loop(|q_js_rt| {
///     let q_ctx = q_js_rt.get_main_context();
///     let obj_ref = q_ctx.eval(Script::new("is_array_test.es", "([1, 2, 3]);")).ok().expect("script failed");
///     let is_array = arrays::is_array_q(q_ctx, &obj_ref);
///     assert!(is_array);
/// });
/// ```
pub fn is_array_q(q_ctx: &QuickJsContext, obj_ref: &JSValueRef) -> bool {
    unsafe { is_array(q_ctx.context, obj_ref) }
}

/// # Safety
/// When passing a context pointer please make sure the corresponding QuickJsContext is still valid
pub unsafe fn is_array(context: *mut q::JSContext, obj_ref: &JSValueRef) -> bool {
    let r = obj_ref.borrow_value();
    let val = q::JS_IsArray(context, *r);
    val > 0
}

/// Get the length of an Array
/// # Example
/// ```rust
/// use quickjs_runtime::esruntimebuilder::EsRuntimeBuilder;
/// use hirofa_utils::js_utils::Script;
/// use quickjs_runtime::quickjs_utils::arrays;
///
/// let rt = EsRuntimeBuilder::new().build();
/// rt.exe_rt_task_in_event_loop(|q_js_rt| {
///     let q_ctx = q_js_rt.get_main_context();
///     let obj_ref = q_ctx.eval(Script::new("get_length_test.es", "([1, 2, 3]);")).ok().expect("script failed");
///     let len = arrays::get_length_q(q_ctx, &obj_ref).ok().expect("could not get length");
///     assert_eq!(len, 3);
/// });
/// ```
pub fn get_length_q(q_ctx: &QuickJsContext, arr_ref: &JSValueRef) -> Result<u32, JsError> {
    unsafe { get_length(q_ctx.context, arr_ref) }
}

/// Get the length of an Array
/// # Safety
/// When passing a context pointer please make sure the corresponding QuickJsContext is still valid
pub unsafe fn get_length(context: *mut q::JSContext, arr_ref: &JSValueRef) -> Result<u32, JsError> {
    let len_ref = crate::quickjs_utils::objects::get_property(context, arr_ref, "length")?;

    let len = crate::quickjs_utils::primitives::to_i32(&len_ref)?;

    Ok(len as u32)
}

/// Create a new Array
/// # Example
/// ```rust
/// use quickjs_runtime::esruntimebuilder::EsRuntimeBuilder;
/// use hirofa_utils::js_utils::Script;
/// use quickjs_runtime::quickjs_utils::{arrays, primitives, functions};
/// use quickjs_runtime::quickjs_utils;
///
/// let rt = EsRuntimeBuilder::new().build();
/// rt.exe_rt_task_in_event_loop(|q_js_rt| {
///     let q_ctx = q_js_rt.get_main_context();
///     // create a method to pass our new array to
///     q_ctx.eval(Script::new("create_array_test.es", "this.create_array_func = function(arr){return arr.length;};")).ok().expect("script failed");
///     // create a new array
///     let arr_ref = arrays::create_array_q(q_ctx).ok().expect("could not create array");
///     // add some values
///     let val0 = primitives::from_i32(12);
///     let val1 = primitives::from_i32(17);
///     arrays::set_element_q(q_ctx, &arr_ref, 0, val0);
///     arrays::set_element_q(q_ctx, &arr_ref, 1, val1);
///     // call the function
///     let result_ref = functions::invoke_member_function_q(q_ctx, &quickjs_utils::get_global_q(q_ctx), "create_array_func", vec![arr_ref]).ok().expect("could not invoke function");
///     let len = primitives::to_i32(&result_ref).ok().unwrap();
///     assert_eq!(len, 2);
/// });
/// ```

pub fn create_array_q(q_ctx: &QuickJsContext) -> Result<JSValueRef, JsError> {
    unsafe { create_array(q_ctx.context) }
}

/// # Safety
/// When passing a context pointer please make sure the corresponding QuickJsContext is still valid
pub unsafe fn create_array(context: *mut q::JSContext) -> Result<JSValueRef, JsError> {
    let arr = q::JS_NewArray(context);
    let arr_ref = JSValueRef::new(context, arr, false, true, "create_array");
    if arr_ref.is_exception() {
        return Err(JsError::new_str("Could not create array in runtime"));
    }
    Ok(arr_ref)
}

/// Set a single element in an array
/// # Example
/// ```rust
/// use quickjs_runtime::esruntimebuilder::EsRuntimeBuilder;
/// use hirofa_utils::js_utils::Script;
/// use quickjs_runtime::quickjs_utils::{arrays, primitives};
/// use quickjs_runtime::quickjs_utils;
///
/// let rt = EsRuntimeBuilder::new().build();
/// rt.exe_rt_task_in_event_loop(|q_js_rt| {
///     let q_ctx = q_js_rt.get_main_context();
///     // get an Array from script
///     let arr_ref = q_ctx.eval(Script::new("set_element_test.es", "([1, 2, 3]);")).ok().expect("script failed");
///     // add some values
///     arrays::set_element_q(q_ctx, &arr_ref, 3, primitives::from_i32(12));
///     arrays::set_element_q(q_ctx, &arr_ref, 4, primitives::from_i32(17));
///     // get the length
///     let len = arrays::get_length_q(q_ctx, &arr_ref).ok().unwrap();
///     assert_eq!(len, 5);
/// });
/// ```
pub fn set_element_q(
    q_ctx: &QuickJsContext,
    array_ref: &JSValueRef,
    index: u32,
    entry_value_ref: JSValueRef,
) -> Result<(), JsError> {
    unsafe { set_element(q_ctx.context, array_ref, index, entry_value_ref) }
}

/// # Safety
/// When passing a context pointer please make sure the corresponding QuickJsContext is still valid
pub unsafe fn set_element(
    context: *mut q::JSContext,
    array_ref: &JSValueRef,
    index: u32,
    entry_value_ref: JSValueRef,
) -> Result<(), JsError> {
    let entry_value_ref = entry_value_ref;

    let ret = q::JS_DefinePropertyValueUint32(
        context,
        *array_ref.borrow_value(),
        index,
        entry_value_ref.clone_value_incr_rc(),
        q::JS_PROP_C_W_E as i32,
    );
    if ret < 0 {
        return Err(JsError::new_str("Could not append element to array"));
    }
    Ok(())
}

/// Get a single element from an array
/// # Example
/// ```rust
/// use quickjs_runtime::esruntimebuilder::EsRuntimeBuilder;
/// use hirofa_utils::js_utils::Script;
/// use quickjs_runtime::quickjs_utils::{arrays, primitives};
/// use quickjs_runtime::quickjs_utils;
///
/// let rt = EsRuntimeBuilder::new().build();
/// rt.exe_rt_task_in_event_loop(|q_js_rt| {
///     let q_ctx = q_js_rt.get_main_context();
///     // get an Array from script
///     let arr_ref = q_ctx.eval(Script::new("get_element_test.es", "([1, 2, 3]);")).ok().expect("script failed");
///     // get a value, the 3 in this case
///     let val_ref = arrays::get_element_q(q_ctx, &arr_ref, 2).ok().unwrap();
///     let val_i32 = primitives::to_i32(&val_ref).ok().unwrap();
///     // get the length
///     assert_eq!(val_i32, 3);
/// });
/// ```
pub fn get_element_q(
    q_ctx: &QuickJsContext,
    array_ref: &JSValueRef,
    index: u32,
) -> Result<JSValueRef, JsError> {
    unsafe { get_element(q_ctx.context, array_ref, index) }
}

/// # Safety
/// When passing a context pointer please make sure the corresponding QuickJsContext is still valid
pub unsafe fn get_element(
    context: *mut q::JSContext,
    array_ref: &JSValueRef,
    index: u32,
) -> Result<JSValueRef, JsError> {
    let value_raw = q::JS_GetPropertyUint32(context, *array_ref.borrow_value(), index);
    let ret = JSValueRef::new(
        context,
        value_raw,
        false,
        true,
        format!("get_element[{}]", index).as_str(),
    );
    if ret.is_exception() {
        return Err(JsError::new_str("Could not build array"));
    }
    Ok(ret)
}

#[cfg(test)]
pub mod tests {
    use crate::esruntime::tests::init_test_rt;
    use crate::esruntime::EsRuntime;
    use crate::quickjs_utils::arrays::{create_array_q, get_element_q, set_element_q};
    use crate::quickjs_utils::objects;
    use std::sync::Arc;

    #[test]
    fn test_array() {
        let rt: Arc<EsRuntime> = init_test_rt();
        rt.exe_rt_task_in_event_loop(|q_js_rt| {
            let q_ctx = q_js_rt.get_main_context();
            let arr = create_array_q(q_ctx).ok().unwrap();
            assert_eq!(arr.get_ref_count(), 1);

            let a = objects::create_object_q(q_ctx).ok().unwrap();
            assert_eq!(1, a.get_ref_count());

            set_element_q(q_ctx, &arr, 0, a.clone()).ok().unwrap();
            assert_eq!(2, a.get_ref_count());

            let a2 = get_element_q(q_ctx, &arr, 0).ok().unwrap();
            assert_eq!(3, a.get_ref_count());
            assert_eq!(3, a2.get_ref_count());
        });
    }
}
