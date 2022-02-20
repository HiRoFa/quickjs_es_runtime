# quickjs_runtime

quickjs_runtime is a library for quickly getting started with embedding a javascript engine in your rust project.

**DISCLAIMER: This project is not yet what I would call "Battle Tested", use at your own risk.**

An example on how to embed a script engine in rust using this lib can be found here: https://github.com/andrieshiemstra/ScriptExtensionLayerExample

quickjs_runtime focuses purely on making [quickjs](https://bellard.org/quickjs/) easy to use and does not add any additional features, that's where these projects come in:
* TypeScript support can be added by using [typescript_utils](https://github.com/HiRoFa/typescript_utils)
* A more feature-rich (e.g. fetch api support, http based module loader and much more) runtime: [GreenCopperRuntime](https://github.com/HiRoFa/GreenCopperRuntime).
* The commandline client: [GreenCopperCmd](https://github.com/HiRoFa/GreenCopperCmd).

This project is inspired by the quickjs wrapper at [theduke/quickjs-rs](https://github.com/theduke/quickjs-rs) and still uses its low level bindings [libquickjs-sys](https://crates.io/crates/libquickjs-sys).

The big difference to quickjs-rs is that quickjs_runtime executes all quickjs related code in a dedicated single-threaded EventLoop.

Please see the [DOCS](https://hirofa.github.io/quickjs_es_runtime/quickjs_runtime/index.html) for all inner workings

# This lib serves two main goals:

## 1. Provide simple utils for working with quickjs (these are located in the quickjs_utils mod)
* The QuickJsRuntime struct, this is to be used from a single thread 
* E.g. objects::set_property(), functions::invoke_func()
* Wrap JSValue to provide reference counting (+1 on init, -1 on drop) ([JSValueRef](https://hirofa.github.io/quickjs_es_runtime/quickjs_runtime/valueref/struct.JSValueRef.html))
* Pass a module loader

## 2. Wrap quickjs for use as a ready to go JavaScript Runtime
* This is the EsRuntime struct, it provides an EventQueue which has a thread_local QuickJsRuntime
* All values are copied or abstracted in an EsValueFacade
* So no need to worry about Garbage collection
* evaluate script and invoke functions while waiting for results blocking or with async/await  
* Get Promise result blocking or with async/await

# What works?

## Script and Modules

* console (.log/info/debug/trace/error) ([docs](https://hirofa.github.io/quickjs_es_runtime/quickjs_runtime/features/console/index.html))
* Eval script ([docs](https://hirofa.github.io/quickjs_es_runtime/hirofa_utils/js_utils/adapters/trait.JsRealmAdapter.html#tymethod.js_eval))
* Create promises in JavaScript which execute async
* Eval modules ([docs](https://hirofa.github.io/quickjs_es_runtime/hirofa_utils/js_utils/adapters/trait.JsRealmAdapter.html#tymethod.js_eval_module))
* Load modules (dynamic and static) ([docs](https://hirofa.github.io/quickjs_es_runtime/hirofa_utils/js_utils/facades/trait.JsRuntimeBuilder.html#tymethod.js_script_module_loader))
* fetch api (moved to [GreenCopperRuntime](https://github.com/HiRoFa/GreenCopperRuntime))
* setImmediate
* setTimeout/Interval (and clear)
* script preprocessing (impls for ifdef/macro's/typescript can be found in [GreenCopperRuntime](https://github.com/HiRoFa/GreenCopperRuntime))

## Rust-Script interoperability

* Return Promises from rust functions and resolve them from rust ([docs](https://hirofa.github.io/quickjs_es_runtime/hirofa_utils/js_utils/adapters/promises/fn.new_resolving_promise.html))
* Add functions from rust ([docs](https://hirofa.github.io/quickjs_es_runtime/quickjs_runtime/quickjsrealmadapter/struct.QuickJsRealmAdapter.html#method.js_install_function))
* Invoke JS functions from rust ([docs](https://hirofa.github.io/quickjs_es_runtime/quickjs_runtime/facades/struct.QuickJsRuntimeFacade.html#method.js_function_invoke))
* Pass primitives, objects and arrays from and to rust ([docs](https://hirofa.github.io/quickjs_es_runtime/quickjs_runtime/quickjs_utils/primitives/index.html))
* Create Classes from rust ([docs](https://hirofa.github.io/quickjs_es_runtime/hirofa_utils/js_utils/adapters/proxies/struct.JsProxy.html))
* async/await support on eval/call_function/promise resolution ([docs](https://hirofa.github.io/quickjs_es_runtime/hirofa_utils/js_utils/facades/values/struct.CachedJsPromiseRef.html#method.js_get_promise_result))
* import native Modules (e.g. dynamic loading of rust functions or Proxy classes) ([docs](https://hirofa.github.io/quickjs_es_runtime/hirofa_utils/js_utils/modules/trait.NativeModuleLoader.html))

## Future / Todo

* Worker support
* WebAssembly support

# goals

Same goals as https://github.com/HiRoFa/es_runtime but with using quickjs

so 
* slower js

but
 
* smaller footprint 
* much faster compilation

For some of my projects those are a big plus!

# examples

Here are some quickstarts:

Cargo.toml

```toml
[dependencies]
hirofa_utils = "0.4"
quickjs_runtime = "0.7"
log = "0.4"
simple-logging = "2.0"
```

```rust
use crate::builder::QuickJsRuntimeBuilder;
    use crate::facades::QuickJsRuntimeFacade;
    use crate::quickjsrealmadapter::QuickJsRealmAdapter;
    use futures::executor::block_on;
    use hirofa_utils::js_utils::adapters::proxies::JsProxy;
    use hirofa_utils::js_utils::adapters::JsRealmAdapter;
    use hirofa_utils::js_utils::facades::values::{JsValueConvertable, JsValueFacade};
    use hirofa_utils::js_utils::facades::{JsRuntimeBuilder, JsRuntimeFacade};
    use hirofa_utils::js_utils::{JsError, Script};
    use log::LevelFilter;
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
        simple_logging::log_to_stderr(LevelFilter::Info);

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
                .ok()
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
```
