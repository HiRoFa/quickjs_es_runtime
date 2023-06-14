# quickjs_runtime

quickjs_runtime is a library for quickly getting started with embedding a javascript engine in your rust project.

**DISCLAIMER: This project is not yet what I would call "Battle Tested", use at your own risk.**

An example on how to embed a script engine in rust using this lib can be found here: [github.com/andrieshiemstra/ScriptExtensionLayerExample](https://github.com/andrieshiemstra/ScriptExtensionLayerExample). It was published in TWIR as a walkthrough. 

quickjs_runtime focuses on making [quickjs](https://bellard.org/quickjs/) easy to use and does not add any additional features, that's where these projects come in:
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

* Typescript (via SWC)
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

# Goals

Embedding a script engine in a rust project seems a very tedious job which involves learning a lot about the inner workings of that engine.

The main goal of this project is to make that job easy!

The manner in which this is achieved is primarily focused on abstracting the workings of the engine from the implementor, therefore some functionality may not be the fastest way of getting things done.

So a second goal is to make implementing a fast and efficient integration doable for the uninitiated, the most common tasks you do with the engine should be doable with the utils in this package and working examples should be provided in the test modules.

The reason I chose QuickJS as the engine is that I've been dealing with less modern engines in my java projects and not being able to use the latest and greatest ECMA-script features becomes quite disappointing at times.

The fun stuff about QuickJS: 
* small footprint 
* fast compilation / startup
* great JS compatibility

# examples

Here are some quickstarts:
* start by reading the [DOCS](https://hirofa.github.io/quickjs_es_runtime/quickjs_runtime/index.html)
* [eval a script](https://hirofa.github.io/quickjs_es_runtime/quickjs_runtime/facades/struct.QuickJsRuntimeFacade.html#method.eval)

The quickjs Api utils:
* [quickjs_utils](https://hirofa.github.io/quickjs_es_runtime/quickjs_runtime/quickjs_utils/index.html)