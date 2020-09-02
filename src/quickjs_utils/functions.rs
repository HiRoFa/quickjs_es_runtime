use crate::eserror::EsError;
use crate::quickjsruntime::{make_cstring, OwnedValueRef, QuickJsRuntime};
use libquickjs_sys as q;

pub fn call_to_string(
    q_js_rt: &QuickJsRuntime,
    obj_ref: &OwnedValueRef,
) -> Result<String, EsError> {
    if obj_ref.is_string() {
        crate::quickjs_utils::primitives::to_string(q_js_rt, obj_ref)
    } else {
        log::trace!("calling JS_ToString on a {}", obj_ref.borrow_value().tag);

        let res = unsafe { q::JS_ToString(q_js_rt.context, *obj_ref.borrow_value()) };
        let res_ref = OwnedValueRef::new(res);

        log::trace!("called JS_ToString got a {}", res_ref.borrow_value().tag);

        if !res_ref.is_string() {
            return Err(EsError::new_str("Could not convert value to string"));
        }
        crate::quickjs_utils::primitives::to_string(q_js_rt, &res_ref)
    }
}

#[allow(dead_code)]
pub fn new_native_function(
    q_js_rt: &QuickJsRuntime,
    name: &str,
    func: q::JSCFunction,
    arg_count: u32,
    is_constructor: bool,
) -> Result<OwnedValueRef, EsError> {
    let cname = make_cstring(name).ok().expect("could not create cstring");
    let magic = 1;

    let cproto = if is_constructor {
        q::JSCFunctionEnum_JS_CFUNC_constructor
    } else {
        q::JSCFunctionEnum_JS_CFUNC_generic
    };

    let func_val = unsafe {
        q::JS_NewCFunction2(
            q_js_rt.context,
            func,
            cname.as_ptr(),
            arg_count as i32,
            cproto,
            magic,
        )
    };
    let func_ref = OwnedValueRef::new(func_val);

    if !func_ref.is_object() {
        return Err(EsError::new_str("Could not create new_native_function"));
    } else {
        Ok(func_ref)
    }
}

#[allow(dead_code)]
pub fn new_native_function_data(
    q_js_rt: &QuickJsRuntime,
    func: q::JSCFunctionData,
    arg_count: u32,
    data: OwnedValueRef,
) -> Result<OwnedValueRef, EsError> {
    let mut data = data;
    let magic = 1;
    let data_len = 1;

    let func_val = unsafe {
        q::JS_NewCFunctionData(
            q_js_rt.context,
            func,
            magic,
            arg_count as i32,
            data_len,
            &mut data.consume_value(),
        )
    };
    let func_ref = OwnedValueRef::new(func_val);

    if !func_ref.is_object() {
        return Err(EsError::new_str("Could not create new_native_function"));
    } else {
        Ok(func_ref)
    }
}

#[allow(dead_code)]
pub fn new_function<F>(
    _q_js_rt: &QuickJsRuntime,
    _name: &str,
    _func: F,
    _arg_count: u32,
) -> Result<OwnedValueRef, EsError>
where
    F: Fn(OwnedValueRef, u32, OwnedValueRef) -> Result<OwnedValueRef, EsError>,
{
    // put func in map, retrieve on call.. todo.. delete on destroy?
    Ok(crate::quickjs_utils::new_null_ref())
}
