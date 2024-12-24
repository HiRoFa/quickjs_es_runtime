//! serialize and stringify JavaScript objects

use crate::jsutils::JsError;
use crate::quickjs_utils;
use crate::quickjsrealmadapter::QuickJsRealmAdapter;
use crate::quickjsvalueadapter::QuickJsValueAdapter;
use libquickjs_sys as q;
use std::ffi::CString;

/// Parse a JSON string into an Object
/// please note that JSON.parse requires member names to be enclosed in double quotes
/// so {a: 1} and {'a': 1} will both fail
/// {"a": 1} will parse ok
/// # Example
/// ```dontrun
/// use quickjs_runtime::esruntimebuilder::EsRuntimeBuilder;
/// use quickjs_runtime::quickjs_utils::{json, objects, primitives};
/// use quickjs_runtime::quickjs_utils::json::parse;
/// let rt = EsRuntimeBuilder::new().build();
/// rt.add_to_event_queue_sync(|q_js_rt| {
///     let q_ctx = q_js_rt.get_main_context();
///     let parse_res = json::parse_q(q_ctx, "{\"aaa\": 165}");
///     if parse_res.is_err() {
///         panic!("could not parse: {}", parse_res.err().unwrap());
///     }
///     let obj_ref = parse_res.ok().unwrap();
///     let a_ref = objects::get_property(q_ctx.context, &obj_ref, "aaa").ok().unwrap();
///     let i = primitives::to_i32(&a_ref).ok().unwrap();
///     assert_eq!(165, i);
/// });
/// rt.gc_sync();
/// ```
pub fn parse_q(q_ctx: &QuickJsRealmAdapter, input: &str) -> Result<QuickJsValueAdapter, JsError> {
    unsafe { parse(q_ctx.context, input) }
}

/// Parse a JSON string into an Object
/// # Safety
/// When passing a context pointer please make sure the corresponding QuickJsContext is still valid
pub unsafe fn parse(
    context: *mut q::JSContext,
    input: &str,
) -> Result<QuickJsValueAdapter, JsError> {
    let s = CString::new(input).ok().unwrap();
    let f_n = CString::new("JSON.parse").ok().unwrap();

    let len = input.len();

    let val = q::JS_ParseJSON(context, s.as_ptr(), len as _, f_n.as_ptr());

    let ret = QuickJsValueAdapter::new(context, val, false, true, "json::parse result");

    if ret.is_exception() {
        if let Some(ex) = QuickJsRealmAdapter::get_exception(context) {
            Err(ex)
        } else {
            Err(JsError::new_str("unknown error while parsing json"))
        }
    } else {
        Ok(ret)
    }
}
/// Stringify an Object in script
/// # Example
/// ```rust
/// use quickjs_runtime::builder::QuickJsRuntimeBuilder;
/// use quickjs_runtime::quickjs_utils::{json, objects, primitives};
/// let rt = QuickJsRuntimeBuilder::new().build();
/// rt.exe_rt_task_in_event_loop(|q_js_rt| {
///     let q_ctx = q_js_rt.get_main_realm();
///     let obj_ref = objects::create_object_q(q_ctx).ok().unwrap();
///     objects::set_property_q(q_ctx, &obj_ref, "a", &primitives::from_i32(741)).ok().unwrap();
///     let str_ref = json::stringify_q(q_ctx, &obj_ref, None).ok().unwrap();
///     let str_str = primitives::to_string_q(q_ctx, &str_ref).ok().unwrap();
///     assert_eq!("{\"a\":741}", str_str);
/// });
/// rt.gc_sync();
/// ```
pub fn stringify_q(
    q_ctx: &QuickJsRealmAdapter,
    input: &QuickJsValueAdapter,
    opt_space: Option<QuickJsValueAdapter>,
) -> Result<QuickJsValueAdapter, JsError> {
    unsafe { stringify(q_ctx.context, input, opt_space) }
}

/// # Safety
/// When passing a context pointer please make sure the corresponding QuickJsContext is still valid
pub unsafe fn stringify(
    context: *mut q::JSContext,
    input: &QuickJsValueAdapter,
    opt_space: Option<QuickJsValueAdapter>,
) -> Result<QuickJsValueAdapter, JsError> {
    //pub fn JS_JSONStringify(
    //         ctx: *mut JSContext,
    //         obj: JSValue,
    //         replacer: JSValue,
    //         space0: JSValue,
    //     ) -> JSValue;

    let space_ref = match opt_space {
        None => quickjs_utils::new_null_ref(),
        Some(s) => s,
    };

    let val = q::JS_JSONStringify(
        context,
        *input.borrow_value(),
        quickjs_utils::new_null(),
        *space_ref.borrow_value(),
    );
    let ret = QuickJsValueAdapter::new(context, val, false, true, "json::stringify result");

    if ret.is_exception() {
        if let Some(ex) = QuickJsRealmAdapter::get_exception(context) {
            Err(ex)
        } else {
            Err(JsError::new_str("unknown error in json::stringify"))
        }
    } else {
        Ok(ret)
    }
}

#[cfg(test)]
pub mod tests {
    use crate::facades::tests::init_test_rt;
    use crate::jsutils::Script;
    use crate::quickjs_utils::json::parse_q;
    use crate::quickjs_utils::{get_global_q, json, objects, primitives};
    use crate::values::JsValueFacade;
    use std::collections::HashMap;

    #[test]
    fn test_json() {
        let rt = init_test_rt();

        log::info!("Starting json test");

        rt.exe_rt_task_in_event_loop(|q_js_rt| {
            let q_ctx = q_js_rt.get_main_realm();

            let obj = objects::create_object_q(q_ctx).ok().unwrap();
            objects::set_property_q(q_ctx, &obj, "a", &primitives::from_i32(532))
                .ok()
                .unwrap();
            objects::set_property_q(q_ctx, &obj, "b", &primitives::from_bool(true))
                .ok()
                .unwrap();
            objects::set_property_q(
                q_ctx,
                &obj,
                "c",
                &primitives::from_string_q(q_ctx, "abcdË").ok().unwrap(),
            )
            .ok()
            .unwrap();
            let str_res = json::stringify_q(q_ctx, &obj, None).ok().unwrap();

            #[cfg(feature = "bellard")]
            assert_eq!(str_res.get_ref_count(), 1);
            let json = str_res.to_string().ok().unwrap();
            assert_eq!(json, "{\"a\":532,\"b\":true,\"c\":\"abcdË\"}");

            let obj2 = parse_q(q_ctx, json.as_str()).ok().unwrap();

            let prop_c = objects::get_property_q(q_ctx, &obj2, "c").ok().unwrap();
            assert_eq!("abcdË", prop_c.to_string().ok().unwrap());
        });
    }

    #[tokio::test]
    async fn test_json_arg() {
        let rt = init_test_rt();

        // init my javascript function
        rt.eval(
            None,
            Script::new(
                "myFunc.js",
                r#"
                function myFunction(argObj) {
                    console.log("I got an %s", typeof argObj);
                    console.log("It looks like this %s", argObj);
                    return "hello " + argObj["key"];
                }
            "#,
            ),
        )
        .await
        .ok()
        .expect("myFunc failed to parse");

        // parse my obj to json
        let mut my_json_deserable_object = HashMap::new();
        my_json_deserable_object.insert("key", "value");
        let json = serde_json::to_string(&my_json_deserable_object)
            .ok()
            .expect("serializing failed");

        let func_res = rt
            .loop_realm(None, move |_rt, realm| {
                // this runs in the worker thread for the EventLoop so json String needs to be moved here
                // now we parse the json to a JsValueRef
                let js_obj = parse_q(realm, json.as_str())
                    .ok()
                    .expect("parsing json failed");
                // then we can invoke the function with that js_obj as input
                // get the global obj as function container
                let global = get_global_q(realm);
                // invoke the function
                let func_res = crate::quickjs_utils::functions::invoke_member_function_q(
                    realm,
                    &global,
                    "myFunction",
                    &[js_obj],
                );
                //return the value out of the worker thread as JsValueFacade
                realm.to_js_value_facade(&func_res.ok().expect("func failed"))
            })
            .await;

        let jsv = func_res.ok().expect("got err");
        assert_eq!(jsv.stringify(), "String: hello value");
    }

    #[tokio::test]
    async fn test_json_arg2() {
        let rt = init_test_rt();

        // init my javascript function
        rt.eval(
            None,
            Script::new(
                "myFunc.js",
                r#"
                function myFunction(argObj) {
                    console.log("I got an %s", typeof argObj);
                    console.log("It looks like this %s", argObj);
                    return "hello " + argObj["key"];
                }
            "#,
            ),
        )
        .await
        .ok()
        .expect("myFunc failed to parse");

        // parse my obj to json
        let mut my_json_deserable_object = HashMap::new();
        my_json_deserable_object.insert("key", "value");
        let json = serde_json::to_string(&my_json_deserable_object)
            .ok()
            .expect("serializing failed");

        let json_js_value_facade = JsValueFacade::JsonStr { json };

        let func_res = rt
            .invoke_function(None, &[], "myFunction", vec![json_js_value_facade])
            .await;

        let jsv = func_res.ok().expect("got err");
        assert_eq!(jsv.stringify(), "String: hello value");
    }
}
