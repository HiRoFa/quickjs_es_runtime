//! # quickjs_runtime
//! This crate consists of two main parts:
//! * thread-safe utils and wrappers
//!   you can call these from any thread, all logic is directed to a single worker-thread(EventLoop) which invokes the quickjs API
//! * quickjs bindings and utils
//!   these talk to the quickjs API directly and need to run in the same thread as the Runtime
//!
//! ## Noteworthy structs
//!
//! These are the structs you'll use the most
//!
//! | Thread safe (Facades) | Runtime Thread-local (Adapters) |
//! | --- | --- | --- | --- |
//! | [QuickJsRuntimeFacade](facades/struct.QuickJsRuntimeFacade.html) the 'starting point' | [QuickJsRuntimeAdapter](quickjsruntimeadapter/struct.QuickJsRuntimeAdapter.html) the wrapper for all things quickjs |
//! | - | [QuickJsRealmAdapter](quickjsrealmadapter/struct.QuickJsRealmAdapter.html) a realm or context |
//! | [JsValueFacade](https://hirofa.github.io/utils/hirofa_utils/js_utils/facades/values/enum.JsValueFacade.html) copy of- or reference to a value in the JsRuntimeAdapter | [QuickJsValueAdapter](quickjsvalueadapter/struct.QuickJsValueAdapter.html) reference counting pointer to a Value |
//!
//! ## Doing something in the runtime worker thread
//!
//! You always start with building a new [QuickjsRuntimeFacade](facades/struct.QuickjsRuntimeFacade.html)
//!
//! ```dontrun
//! use quickjs_runtime::builder::QuickJsRuntimeBuilder;
//! let rt: JsRuntimeFacade = QuickJsRuntimeBuilder::new().js_build();
//! ```
//!
//! [QuickJsRuntimeFacade](facades/struct.QuickJsRuntimeFacade.html) has plenty public methods you can check out but one of the things you'll need to understand is how to communicate with the [QuickJsRuntimeAdapter](quickjsruntimeadapter/struct.QuickJsRuntimeAdapter.html) and the [QuickJsRealmAdapter](quickjsrealmadapter/struct.QuickJsRealmAdapter.html)
//! This is done by adding a job to the [EventLoop](https://hirofa.github.io/utils/hirofa_utils/eventloop/struct.EventLoop.html) of the [QuickJsRuntimeFacade](facades/struct.QuickJsRuntimeFacade.html)
//!
//! ```dontrun
//! // with the first Option you may specify which realm to use, None indicates the default or main realm
//! let res = rt.loop_realm(None, |rt: QuickJsRuntimeAdapter, realm: QuickJsRealmAdapter| {
//!    // this will run in the Worker thread, here we can use the Adapters
//!    // since we passed None as realm the realm adapter will be the "main" realm
//!    return true;
//! }).await;
//! ```
//! All the non-sync functions return a Future so you can .await them from async functions.
//!
//! In order to do something and get the result synchronously you can use the sync variant
//! ```dontrun
//! use quickjs_runtime::quickjsruntime::QuickJsRuntime;
//! let res = rt.loop_realm_sync(None, |rt, realm| {
//!    // this will run in the Worker thread, here we can use the quickjs API
//!    return 1;
//! });
//! ```
//!
//! One last thing you need to know is how to pass values from the js engine out of the worker thread
//!
//! This is where the JsValueFacade comes in
//!
//! ```dontrun
//!
//! // init a simple function
//! rt.eval(Script::new("init_func.js", "globalThis.myObj = {someMember: {someFunction: function(input){return(input + " > hello rust!");}}};")).await;
//!
//! // create an input variable by using one of the constructor methods of the JsValueFacade
//! let input_facade = JsValueFacade::new_str("hello js!");
//! // move it into a closure which will run in the worker thread
//! let res = rt.loop_realm(None, move |rt: JsRuntimeAdapter, realm: JsRealmAdapter| {
//!    // convert the input JsValueFacade to JsValueAdapter
//!    let input_adapter = realm.from_js_value_facade(input_facade);
//!    // call myObj.someMember.someFunction();
//!    let result_adapter = realm.invoke_function_by_name(&["myObj", "someMember"], "someFunction", &[input_adapter])?;
//!    // convert adapter to facade again so it may move out of the worker thread
//!    return realm.to_js_value_facade(&result_adapter);
//! }).await;
//! assert_eq!(res.get_str(), "hello_js! > hello rust!");
//! ```
//!
//! For more details and examples please explore the packages below

#[macro_use]
extern crate lazy_static;
extern crate core;

pub mod builder;
pub mod facades;
#[cfg(any(
    feature = "settimeout",
    feature = "setinterval",
    feature = "console",
    feature = "setimmediate"
))]
pub mod features;
pub mod jsutils;
pub mod quickjs_utils;
pub mod quickjsrealmadapter;
pub mod quickjsruntimeadapter;
pub mod quickjsvalueadapter;
pub mod reflection;
#[cfg(feature = "typescript")]
pub mod typescript;
pub mod values;

#[cfg(test)]
pub mod tests {
    use crate::builder::QuickJsRuntimeBuilder;
    use crate::facades::QuickJsRuntimeFacade;
    use crate::jsutils::jsproxies::JsProxy;
    use crate::jsutils::{JsError, Script};
    use crate::quickjsrealmadapter::QuickJsRealmAdapter;
    use crate::values::{JsValueConvertable, JsValueFacade};
    use futures::executor::block_on;
    use std::time::Duration;

    #[test]
    fn test_examples() {
        let rt = QuickJsRuntimeBuilder::new().build();
        let outcome = block_on(run_examples(&rt));
        if outcome.is_err() {
            log::error!("an error occured: {}", outcome.err().unwrap());
        }
        log::info!("done");
    }

    async fn take_long() -> i32 {
        std::thread::sleep(Duration::from_millis(500));
        537
    }

    async fn run_examples(rt: &QuickJsRuntimeFacade) -> Result<(), JsError> {
        // ensure console.log calls get outputted
        //simple_logging::log_to_stderr(LevelFilter::Info);

        // do a simple eval on the main realm
        let eval_res = rt.eval(None, Script::new("simple_eval.js", "2*7;")).await?;
        log::info!("simple eval:{}", eval_res.get_i32());

        // invoke a JS method from rust

        let meth_res = rt
            .invoke_function(None, &["Math"], "round", vec![12.321.to_js_value_facade()])
            .await?;
        log::info!("Math.round(12.321) = {}", meth_res.get_i32());

        // add a rust function to js as a callback

        let cb = JsValueFacade::new_callback(|args| {
            let a = args[0].get_i32();
            let b = args[1].get_i32();
            log::info!("rust cb was called with a:{} and b:{}", a, b);
            Ok(JsValueFacade::Null)
        });
        rt.invoke_function(
            None,
            &[],
            "setTimeout",
            vec![
                cb,
                10.to_js_value_facade(),
                12.to_js_value_facade(),
                13.to_js_value_facade(),
            ],
        )
        .await?;
        std::thread::sleep(Duration::from_millis(20));
        log::info!("rust cb should have been called by now");

        // create simple proxy class with an async function
        rt.loop_realm_sync(None, |_rt_adapter, realm_adapter| {
            let proxy = JsProxy::new()
                .namespace(&["com", "mystuff"])
                .name("MyProxy")
                .static_method(
                    "doSomething",
                    |_rt_adapter, realm_adapter: &QuickJsRealmAdapter, _args| {
                        realm_adapter.create_resolving_promise_async(
                            async { Ok(take_long().await) },
                            |realm_adapter, producer_result| {
                                realm_adapter.create_i32(producer_result)
                            },
                        )
                    },
                );
            realm_adapter
                .install_proxy(proxy, true)
                .expect("could not install proxy");
        });

        rt.eval(
            None,
            Script::new(
                "testMyProxy.js",
                "async function a() {\
                            console.log('a called at %s ms', new Date().getTime());\
                            let res = await com.mystuff.MyProxy.doSomething();\
                            console.log('a got result %s at %s ms', res, new Date().getTime());\
                           }; a();",
            ),
        )
        .await?;
        std::thread::sleep(Duration::from_millis(600));
        log::info!("a should have been called by now");

        Ok(())
    }
}
