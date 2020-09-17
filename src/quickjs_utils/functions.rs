use crate::eserror::EsError;
use crate::quickjsruntime::{make_cstring, OwnedValueRef, QuickJsRuntime};
use libquickjs_sys as q;

#[allow(dead_code)]
pub fn call_function(
    q_js_rt: &QuickJsRuntime,
    function_ref: &OwnedValueRef,
    arguments: &Vec<OwnedValueRef>,
) -> Result<OwnedValueRef, EsError> {
    assert!(is_function(q_js_rt, function_ref));

    let arg_count = arguments.len() as i32;

    let mut qargs = arguments
        .iter()
        .map(|arg| *arg.borrow_value())
        .collect::<Vec<_>>();

    let this_ref = crate::quickjs_utils::new_null_ref();

    let res = unsafe {
        q::JS_Call(
            q_js_rt.context,
            *function_ref.borrow_value(),
            *this_ref.borrow_value(), // this todo
            arg_count,
            qargs.as_mut_ptr(),
        )
    };

    let res_ref = OwnedValueRef::new(res);

    if res_ref.is_exception() {
        if let Some(ex) = q_js_rt.get_exception() {
            Err(ex)
        } else {
            Err(EsError::new_str(
                "function invocation failed but could not get ex",
            ))
        }
    } else {
        Ok(res_ref)
    }
}

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
pub fn is_function(q_js_rt: &QuickJsRuntime, obj_ref: &OwnedValueRef) -> bool {
    if obj_ref.is_object() {
        let res = unsafe { q::JS_IsFunction(q_js_rt.context, *obj_ref.borrow_value()) };
        res != 0
    } else {
        false
    }
}

#[allow(dead_code)]
pub fn is_constructor(q_js_rt: &QuickJsRuntime, obj_ref: &OwnedValueRef) -> bool {
    if obj_ref.is_object() {
        let res = unsafe { q::JS_IsConstructor(q_js_rt.context, *obj_ref.borrow_value()) };
        res != 0
    } else {
        false
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

#[cfg(test)]
pub mod tests {
    use crate::esruntime::EsRuntime;
    use crate::esscript::EsScript;
    use crate::quickjs_utils::functions::{call_function, call_to_string};
    use crate::quickjs_utils::primitives;
    use std::sync::Arc;

    #[test]
    pub fn test_to_string() {
        let rt: Arc<EsRuntime> = crate::esruntime::tests::TEST_ESRT.clone();
        let io = rt.add_to_event_queue_sync(|q_js_rt| {
            let i = primitives::from_i32(480);
            let i_s = call_to_string(q_js_rt, &i)
                .ok()
                .expect("to_string failed on i");
            assert_eq!(i_s.as_str(), "480");

            let b = primitives::from_bool(true);
            let b_s = call_to_string(q_js_rt, &b)
                .ok()
                .expect("to_string failed on b");
            assert_eq!(b_s.as_str(), "true");

            true
        });
        assert!(io);
    }

    #[test]
    pub fn test_call() {
        let rt: Arc<EsRuntime> = crate::esruntime::tests::TEST_ESRT.clone();
        let io = rt.add_to_event_queue_sync(|q_js_rt| {
            let func_ref = q_js_rt
                .eval(EsScript::new(
                    "test_call.es".to_string(),
                    "(function(a, b){return ((a || 7)*(b || 7));});".to_string(),
                ))
                .ok()
                .expect("could not get func obj");

            let res = call_function(
                q_js_rt,
                &func_ref,
                &vec![primitives::from_i32(8), primitives::from_i32(6)],
            );
            if res.is_err() {
                panic!("test_call failed: {}");
            }
            let res_val = res.ok().unwrap();

            assert!(res_val.is_i32());

            assert_eq!(primitives::to_i32(&res_val).ok().unwrap(), 6 * 8);

            true
        });
        assert!(io)
    }
}
