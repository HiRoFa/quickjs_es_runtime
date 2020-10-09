use crate::eserror::EsError;
use crate::quickjsruntime::QuickJsRuntime;
use crate::valueref::JSValueRef;
use libquickjs_sys as q;

/// Check whether an object is an array
/// # Example
/// ```rust
/// use quickjs_es_runtime::esruntimebuilder::EsRuntimeBuilder;
/// use quickjs_es_runtime::esscript::EsScript;
/// use quickjs_es_runtime::quickjs_utils::arrays;
///
/// let rt = EsRuntimeBuilder::new().build();
/// rt.add_to_event_queue_sync(|q_js_rt| {
///     let obj_ref = q_js_rt.eval(EsScript::new("is_array_test.es", "([1, 2, 3]);")).ok().expect("script failed");
///     let is_array = arrays::is_array(q_js_rt, &obj_ref);
///     assert!(is_array);
/// });
/// ```
pub fn is_array(q_js_rt: &QuickJsRuntime, obj_ref: &JSValueRef) -> bool {
    let r = obj_ref.borrow_value();
    let val = unsafe { q::JS_IsArray(q_js_rt.context, *r) };
    val > 0
}

/// Get the length of an Array
/// # Example
/// ```rust
/// use quickjs_es_runtime::esruntimebuilder::EsRuntimeBuilder;
/// use quickjs_es_runtime::esscript::EsScript;
/// use quickjs_es_runtime::quickjs_utils::arrays;
///
/// let rt = EsRuntimeBuilder::new().build();
/// rt.add_to_event_queue_sync(|q_js_rt| {
///     let obj_ref = q_js_rt.eval(EsScript::new("get_length_test.es", "([1, 2, 3]);")).ok().expect("script failed");
///     let len = arrays::get_length(q_js_rt, &obj_ref).ok().expect("could not get length");
///     assert_eq!(len, 3);
/// });
/// ```
pub fn get_length(q_js_rt: &QuickJsRuntime, arr_ref: &JSValueRef) -> Result<u32, EsError> {
    let len_ref = crate::quickjs_utils::objects::get_property(q_js_rt, arr_ref, "length")?;

    let len = crate::quickjs_utils::primitives::to_i32(&len_ref)?;

    Ok(len as u32)
}

/// Create a new Array
/// # Example
/// ```rust
/// use quickjs_es_runtime::esruntimebuilder::EsRuntimeBuilder;
/// use quickjs_es_runtime::esscript::EsScript;
/// use quickjs_es_runtime::quickjs_utils::{arrays, primitives, functions};
/// use quickjs_es_runtime::quickjs_utils;
///
/// let rt = EsRuntimeBuilder::new().build();
/// rt.add_to_event_queue_sync(|q_js_rt| {
///     // create a method to pass our new array to
///     q_js_rt.eval(EsScript::new("create_array_test.es", "this.create_array_func = function(arr){return arr.length;};")).ok().expect("script failed");
///     // create a new array
///     let arr_ref = arrays::create_array(q_js_rt).ok().expect("could not create array");
///     // add some values
///     let val0 = primitives::from_i32(12);
///     let val1 = primitives::from_i32(17);
///     arrays::set_element(q_js_rt, &arr_ref, 0, val0);
///     arrays::set_element(q_js_rt, &arr_ref, 1, val1);
///     // call the function
///     let result_ref = functions::invoke_member_function(q_js_rt, &quickjs_utils::get_global(q_js_rt), "create_array_func", vec![arr_ref]).ok().expect("could not invoke function");
///     let len = primitives::to_i32(&result_ref).ok().unwrap();
///     assert_eq!(len, 2);
/// });
/// ```
pub fn create_array(q_js_rt: &QuickJsRuntime) -> Result<JSValueRef, EsError> {
    let arr = unsafe { q::JS_NewArray(q_js_rt.context) };
    let arr_ref = JSValueRef::new_no_ref_ct_increment(arr, "create_array");
    if arr_ref.is_exception() {
        return Err(EsError::new_str("Could not create array in runtime"));
    }
    Ok(arr_ref)
}

/// Set a single element in an array
/// # Example
/// ```rust
/// use quickjs_es_runtime::esruntimebuilder::EsRuntimeBuilder;
/// use quickjs_es_runtime::esscript::EsScript;
/// use quickjs_es_runtime::quickjs_utils::{arrays, primitives};
/// use quickjs_es_runtime::quickjs_utils;
///
/// let rt = EsRuntimeBuilder::new().build();
/// rt.add_to_event_queue_sync(|q_js_rt| {
///     // get an Array from script
///     let arr_ref = q_js_rt.eval(EsScript::new("set_element_test.es", "([1, 2, 3]);")).ok().expect("script failed");
///     // add some values
///     arrays::set_element(q_js_rt, &arr_ref, 3, primitives::from_i32(12));
///     arrays::set_element(q_js_rt, &arr_ref, 4, primitives::from_i32(17));
///     // get the length
///     let len = arrays::get_length(q_js_rt, &arr_ref).ok().unwrap();
///     assert_eq!(len, 5);
/// });
/// ```
pub fn set_element(
    q_js_rt: &QuickJsRuntime,
    array_ref: &JSValueRef,
    index: u32,
    entry_value_ref: JSValueRef,
) -> Result<(), EsError> {
    let entry_value_ref = entry_value_ref;

    let ret = unsafe {
        q::JS_DefinePropertyValueUint32(
            q_js_rt.context,
            *array_ref.borrow_value(),
            index,
            entry_value_ref.consume_value_no_decr_rc(),
            q::JS_PROP_C_W_E as i32,
        )
    };
    if ret < 0 {
        return Err(EsError::new_str("Could not append element to array"));
    }
    Ok(())
}

/// Get a single element from an array
/// # Example
/// ```rust
/// use quickjs_es_runtime::esruntimebuilder::EsRuntimeBuilder;
/// use quickjs_es_runtime::esscript::EsScript;
/// use quickjs_es_runtime::quickjs_utils::{arrays, primitives};
/// use quickjs_es_runtime::quickjs_utils;
///
/// let rt = EsRuntimeBuilder::new().build();
/// rt.add_to_event_queue_sync(|q_js_rt| {
///     // get an Array from script
///     let arr_ref = q_js_rt.eval(EsScript::new("get_element_test.es", "([1, 2, 3]);")).ok().expect("script failed");
///     // get a value, the 3 in this case
///     let val_ref = arrays::get_element(q_js_rt, &arr_ref, 2).ok().unwrap();
///     let val_i32 = primitives::to_i32(&val_ref).ok().unwrap();
///     // get the length
///     assert_eq!(val_i32, 3);
/// });
/// ```
pub fn get_element(
    q_js_rt: &QuickJsRuntime,
    array_ref: &JSValueRef,
    index: u32,
) -> Result<JSValueRef, EsError> {
    let value_raw =
        unsafe { q::JS_GetPropertyUint32(q_js_rt.context, *array_ref.borrow_value(), index) };
    let ret =
        JSValueRef::new_no_ref_ct_increment(value_raw, format!("get_element[{}]", index).as_str());
    if ret.is_exception() {
        return Err(EsError::new_str("Could not build array"));
    }
    Ok(ret)
}

#[cfg(test)]
pub mod tests {
    use crate::esruntime::EsRuntime;
    use crate::quickjs_utils::arrays::{create_array, get_element, set_element};
    use crate::quickjs_utils::objects;
    use std::sync::Arc;

    #[test]
    fn test_array() {
        let rt: Arc<EsRuntime> = crate::esruntime::tests::TEST_ESRT.clone();
        rt.add_to_event_queue_sync(|q_js_rt| {
            let arr = create_array(q_js_rt).ok().unwrap();
            assert_eq!(arr.get_ref_count(), 1);

            let a = objects::create_object(q_js_rt).ok().unwrap();
            assert_eq!(1, a.get_ref_count());

            set_element(q_js_rt, &arr, 0, a.clone()).ok().unwrap();
            assert_eq!(2, a.get_ref_count());

            let a2 = get_element(q_js_rt, &arr, 0).ok().unwrap();
            assert_eq!(3, a.get_ref_count());
            assert_eq!(3, a2.get_ref_count());
        });
    }
}
