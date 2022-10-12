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
//! | Thread safe | Abstracted in hirofa_utils as | Runtime Thread-local | Abstracted in hirofa_utils as |
//! | --- | --- | --- | --- |
//! | [QuickjsRuntimeFacade](facades/struct.QuickjsRuntimeFacade.html) the 'starting point' | [JsRuntimeFacade](https://hirofa.github.io/utils/hirofa_utils/js_utils/facades/trait.JsRuntimeFacade.html) | [QuickJsRuntimeAdapter](quickjsruntimeadapter/struct.QuickJsRuntimeAdapter.html) the wrapper for all things quickjs | [JsRuntimeAdapter](https://hirofa.github.io/utils/hirofa_utils/js_utils/adapters/trait.JsRuntimeAdapter.html) |
//! | - | - | [QuickJsRealmAdapter](quickjsrealmadapter/struct.QuickJsRealmAdapter.html) a realm or context | [JsRealmAdapter](https://hirofa.github.io/utils/hirofa_utils/js_utils/adapters/trait.JsRealmAdapter.html) |
//! | - | [JsValueFacade](https://hirofa.github.io/utils/hirofa_utils/js_utils/facades/values/enum.JsValueFacade.html) copy of- or reference to a value in the JsRuntimeAdapter | [JSValueRef](valueref/struct.JSValueRef.html) reference counting pointer to a Value | [JsValueAdapter](https://hirofa.github.io/utils/hirofa_utils/js_utils/adapters/trait.JsValueAdapter.html) |
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
//! [JsRuntimeFacade](https://hirofa.github.io/utils/hirofa_utils/js_utils/facades/trait.JsRuntimeFacade.html) has plenty public methods you can check out but one of the things you'll need to understand is how to communicate with the [JsRuntimeAdapter](https://hirofa.github.io/utils/hirofa_utils/js_utils/adapters/trait.JsRuntimeAdapter.html) and the [JsRealmAdapter](https://hirofa.github.io/utils/hirofa_utils/js_utils/adapters/trait.JsRealmAdapter.html)
//! This is done by adding a job to the [EventLoop](https://hirofa.github.io/utils/hirofa_utils/eventloop/struct.EventLoop.html) of the [JsRuntimeFacade](https://hirofa.github.io/utils/hirofa_utils/js_utils/facades/trait.JsRuntimeFacade.html)
//!
//! ```dontrun
//! // with the first Option you may specify which realm to use, None indicates the default or main realm
//! let res = rt.js_loop_realm(None, |rt: JsRuntimeAdapter, realm: JsRealmAdapter| {
//!    // this will run in the Worker thread, here we can use the Adapters
//!    
//!    return true;
//! }).await;
//! ```
//! All the non-sync functions return a Future so you can .await them from async functions.
//!
//! In order to do something and get the result synchronously you can use the sync variant
//! ```dontrun
//! use quickjs_runtime::quickjsruntime::QuickJsRuntime;
//! let res = rt.js_loop_realm_sync(None, |rt, realm| {
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
//! rt.js_eval(Script::new("init_func.js", "globalThis.myObj = {someMember: {someFunction: function(input){return(input + " > hello rust!");}}};")).await;
//!
//! let input_facade = JsValueFacade::new_str("hello js!");
//! let res = rt.js_loop_realm(None, move |rt: JsRuntimeAdapter, realm: JsRealmAdapter| {
//!    // convert JsValueFacade to JsValueAdapter
//!    let input_adapter = realm.from_js_value_facade(input_facade);
//!    // call myObj.someMember.someFunction();
//!    let result_adapter = realm.js_function_invoke_by_name(&["myObj", "someMember"], "someFunction", &[input_adapter])?;
//!    // convert adapter to facade again so it may move out of the worker thread
//!    return realm.to_js_value_facade();
//! }).await;
//! assert_eq!(res.get_str(), "hello_js! > hello rust!");
//! ```
//!
//! For more details and examples please explore the packages below

#[macro_use]
extern crate lazy_static;
extern crate core;

#[macro_export]
macro_rules! es_args {
    ( $( $x:expr ),* ) => {
        {
            let mut temp_vec: Vec<$crate::esvalue::EsValueFacade> = Vec::new();
            $(
                temp_vec.push($crate::esvalue::EsValueConvertible::to_es_value_facade($x));
            )*
            temp_vec
        }
    };
}

pub mod builder;
pub mod esvalue;
pub mod facades;
#[cfg(any(
    feature = "settimeout",
    feature = "setinterval",
    feature = "console",
    feature = "setimmediate"
))]
pub mod features;
pub mod quickjs_utils;
pub mod quickjsrealmadapter;
pub mod quickjsruntimeadapter;
pub mod reflection;
pub mod runtimefacade_utils;
pub mod valueref;

#[cfg(test)]
pub mod tests {
    use crate::builder::QuickJsRuntimeBuilder;
    use crate::facades::QuickJsRuntimeFacade;
    use crate::quickjsrealmadapter::QuickJsRealmAdapter;
    use futures::executor::block_on;
    use hirofa_utils::js_utils::adapters::proxies::JsProxy;
    use hirofa_utils::js_utils::adapters::JsRealmAdapter;
    use hirofa_utils::js_utils::facades::values::{JsValueConvertable, JsValueFacade};
    use hirofa_utils::js_utils::facades::{JsRuntimeBuilder, JsRuntimeFacade};
    use hirofa_utils::js_utils::{JsError, Script};
    use std::time::Duration;

    #[test]
    fn test_examples() {
        let rt = QuickJsRuntimeBuilder::new().js_build();
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
        let eval_res = rt
            .js_eval(None, Script::new("simple_eval.js", "2*7;"))
            .await?;
        log::info!("simple eval:{}", eval_res.get_i32());

        // invoke a JS method from rust

        let meth_res = rt
            .js_function_invoke(None, &["Math"], "round", vec![12.321.to_js_value_facade()])
            .await?;
        log::info!("Math.round(12.321) = {}", meth_res.get_i32());

        // add a rust function to js as a callback

        let cb = JsValueFacade::new_callback(|args| {
            let a = args[0].get_i32();
            let b = args[1].get_i32();
            log::info!("rust cb was called with a:{} and b:{}", a, b);
            Ok(JsValueFacade::Null)
        });
        rt.js_function_invoke(
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
        rt.js_loop_realm_sync(None, |_rt_adapter, realm_adapter| {
            let proxy = JsProxy::new(&["com", "mystuff"], "MyProxy").add_static_method(
                "doSomething",
                |_rt_adapter, realm_adapter: &QuickJsRealmAdapter, _args| {
                    realm_adapter.js_promise_create_resolving_async(
                        async { Ok(take_long().await) },
                        |realm_adapter, producer_result| {
                            realm_adapter.js_i32_create(producer_result)
                        },
                    )
                },
            );
            realm_adapter
                .js_proxy_install(proxy, true)
                .expect("could not install proxy");
        });

        rt.js_eval(
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
