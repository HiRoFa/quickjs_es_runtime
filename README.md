# quickjs_runtime

quickjs_runtime is a library for quickly getting started with embedding a javascript engine in your rust project.

**DISCLAIMER: This project is far from what I would call "Battle Tested", use at your own risk.**

quickjs_runtime focuses purely on making [quickjs](https://bellard.org/quickjs/) easy to use and does not add anny additional features, that where these projects come in:
* A more feature-rich runtime can be found in [GreenCopperRuntime](https://github.com/HiRoFa/GreenCopperRuntime).
* There is also a commandline client: [GreenCopperCmd](https://github.com/HiRoFa/GreenCopperCmd).
* And last but not least there is GreenCopper which aspires to be a full fledged application platform: [GreenCopperServer](https://github.com/HiRoFa/GreenCopperServer).

This project is heavily inspired by the awesome quickjs wrapper at [theduke/quickjs-rs](https://github.com/theduke/quickjs-rs) and still uses its low level bindings [libquickjs-sys](https://crates.io/crates/libquickjs-sys).

The big difference to quickjs-rs is that quickjs_runtime executes all quickjs related code in a dedicated single-threaded EventQueue.

Please see the [DOCS](https://hirofa.github.io/quickjs_es_runtime/quickjs_runtime/index.html) for all inner workings

# This lib serves two main goals:

## 1. Provide simple utils for working with quickjs (these are located in the quickjs_utils mod)
* The QuickJSRuntime struct, this is to be used from a single thread 
* E.g. objects::set_property(), functions::invoke_func()
* Wrap JSValue to provide reference counting (+1 on init, -1 on drop)
* Pass a module loader

## 2. Wrap quickjs for use as a ready to go JavaScript Runtime
* This is the ESRuntime struct, it provides an EventQueue which has a thread_local QuickJSRuntime
* All values are copied or abstracted in an ESValueFacade
* So no need to worry about Garbage collection
* evaluate script and invoke functions while waiting for results blocking or with async/await  
* Get Promise result blocking or with async/await

# What works?

## Script and Modules

* console (.log/info/debug/trace/error) ([docs](https://hirofa.github.io/quickjs_es_runtime/quickjs_runtime/features/console/index.html))
* Eval script ([docs](https://hirofa.github.io/quickjs_es_runtime/quickjs_runtime/esruntime/struct.EsRuntime.html#method.eval_sync))
* Create promises in JavaScript which execute async
* Eval modules ([docs](https://hirofa.github.io/quickjs_es_runtime/quickjs_runtime/esruntime/struct.EsRuntime.html#method.eval_module_sync))
* Load modules (dynamic and static) ([docs](https://hirofa.github.io/quickjs_es_runtime/quickjs_runtime/esruntimebuilder/struct.EsRuntimeBuilder.html#method.module_script_loader))
* fetch api ([docs](https://hirofa.github.io/quickjs_es_runtime/quickjs_runtime/esruntimebuilder/struct.EsRuntimeBuilder.html#method.fetch_response_provider))
* setImmediate
* setTimeout/Interval (and clear)

## Rust-Script interoperability

* Return Promises from rust functions and resolve them from rust ([docs](https://hirofa.github.io/quickjs_es_runtime/quickjs_runtime/esruntime_utils/promises/fn.new_resolving_promise.html))
* Add functions from rust ([docs](https://hirofa.github.io/quickjs_es_runtime/quickjs_runtime/esruntime/struct.EsRuntime.html#method.set_function))
* Invoke JS functions from rust ([docs](https://hirofa.github.io/quickjs_es_runtime/quickjs_runtime/esruntime/struct.EsRuntime.html#method.call_function_sync))
* Pass primitives, objects and arrays from and to rust ([docs](https://hirofa.github.io/quickjs_es_runtime/quickjs_runtime/quickjs_utils/primitives/index.html))
* Create Classes from rust ([docs](https://hirofa.github.io/quickjs_es_runtime/quickjs_runtime/reflection/struct.Proxy.html))
* async/await support on eval/call_function/promise resolution ([docs](https://hirofa.github.io/quickjs_es_runtime/quickjs_runtime/esvalue/struct.EsValueFacade.html#method.get_promise_result))
* import native Modules (e.g. dynamic loading of rust functions or Proxy classes) ([docs](https://hirofa.github.io/quickjs_es_runtime/quickjs_runtime/esruntimebuilder/struct.EsRuntimeBuilder.html#method.native_module_loader))

## Future / Todo

* Worker support
* WebAssembly support
* Macro / IFDef support
* Pre processing (for e.g. typescript)

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
quickjs_runtime = "0.2"
log = "0.4.11"
simple-logging = "2.0.2"
```

main.rs

```rust
use quickjs_runtime::esruntimebuilder::EsRuntimeBuilder;
use quickjs_runtime::esscript::EsScript;
use log::LevelFilter;
use futures::executor::block_on;
use std::sync::Arc;
use quickjs_runtime::esruntime::EsRuntime;

async fn test(rt: Arc<EsRuntime>){
    let res = rt.eval(EsScript::new(
        "basics.es",
        "7 * 12;",
    )).await.ok().unwrap();

    assert_eq!(res.get_i32(), 84);
}

fn main() {
    simple_logging::log_to_stderr(LevelFilter::Info);

    let rt = EsRuntimeBuilder::new()
        .build();

    block_on(test(rt));

}
```

## invoke a js method from rust

```rust
    let res = rt.call_function(vec!["myAppUtils"], "some_func", es_args![8, 7]).await;
    match res {
        Ok(val) => log::info!("8*7 in JavaScript = {}", val.get_i32()),
        Err(e) => println!("script failed: {}", e),
    }
```

## add a function from rust and invoke it

```rust
    rt.set_function(vec!["nl", "my", "utils"], "methodA", |args| {
        if args.len() != 2 || !args.get(0).unwrap().is_i32() || !args.get(1).unwrap().is_i32() {
            Err(EsError::new_str(
                "i'd really like 2 args of the int32 kind please",
            ))
        } else {
            let a = args[0].get_i32();
            let b = args[1].get_i32();
            log::info!("rust is multiplying {} and {}", a, b);
            Ok((a * b).to_es_value_facade())
        }
    })
    .ok()
    .expect("set_function failed");

    let method_a_res = rt.eval(EsScript::new(
        "test_func.es",
        "(nl.my.utils.methodA(13, 56));",
    )).await;

    match method_a_res {
        Ok(val) => {
            assert_eq!(val.get_i32(), 13 * 56);
        }
        Err(e) => {
            panic!("test_func.es failed: {}", e);
        }
    }
```

## eval a module

```rust
    rt.eval_module(EsScript::new(
        "my_app.mes",
        "\
    import {foo} from 'example.mes';\
    console.log('static foo is ' + foo);\
    ",
    )).await
    .ok()
    .expect("module failed");
```

## eval a module with a dynamic import

```rust
    
    rt.eval_module(EsScript::new(
        "my_app2.es",
        "\
    import('example.mes')\
    .then((example_module) => {\
        console.log('dynamic foo is ' + example_module.foo);\
    });\
    ",
    )).await
    .ok()
    .expect("script failed");
```

##  get a function from js and invoke it in rust

```rust
    rt.set_function(vec!["nl", "my", "utils"], "methodB", |mut args| {
        if args.len() != 1 || !args[0].is_function() {
            Err(EsError::new_str(
                "i'd really like 1 arg of the function kind please",
            ))
        } else {
            let consumer_func = args.remove(0);

            // invoke the func async, just because we can
            std::thread::spawn(move || {
                consumer_func
                    .invoke_function_sync(es_args![19, 17])
                    .ok()
                    .expect("func failed");
            });

            Ok(quickjs_es_runtime::esvalue::EsNullValue {}.to_es_value_facade())
        }
    })
    .ok()
    .expect("set_function failed");

    rt.eval(EsScript::new(
        "test_func2.es",
        "(nl.my.utils.methodB(function(a, b){console.log('consumer was called with ' +a + ', ' + b);}));",
    )).await.ok().expect("test_func2.es failed");

    // wait a sec for the async onvoker to run
    std::thread::sleep(Duration::from_secs(1));
```
