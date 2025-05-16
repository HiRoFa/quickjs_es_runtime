# quickjs_runtime

Quickjs_runtime is a library for quickly getting started with embedding a javascript engine in your rust project.

Relies on [hirofa-quickjs-sys](https://github.com/HiRoFa/quickjs-sys) to support quickjs-ng as well as the original
quickjs

Quickjs_runtime runs all javascript action in a single thread using an EventLoop. This means you can call javascript
safely from several threads by adding tasks to the EventLoop.

# quickjs or quickjs-ng

Quickjs_runtime supports both the original quickjs and the quickjs-ng project.

You can use quickjs-ng by adding the dep to quickjs_runtime like this:

```toml
quickjs_runtime = { version = "0.15", features = ["console", "setimmediate", "setinterval", "settimeout", "typescript", "quickjs-ng"], default-features = false }
```

# OS support

| features       | linux | mac | windows        |
|----------------|-------|-----|----------------|
| **bellard**    | yes   | yes | mingW only     |
| **quickjs-ng** | yes   | yes | mingW and MSVC |

# Usage and Features

An example on how to embed a script engine in rust using this lib can be found
here: [github.com/andrieshiemstra/ScriptExtensionLayerExample](https://github.com/andrieshiemstra/ScriptExtensionLayerExample).
It was published in TWIR as a walkthrough.

Quickjs_runtime focuses on making [quickjs](https://bellard.org/quickjs/) easy to use and does not add any additional
features, that's where these projects come in:

* A more feature-rich (e.g. fetch api support, http based module loader and much more)
  runtime: [GreenCopperRuntime](https://github.com/HiRoFa/GreenCopperRuntime).
* The commandline client: [GreenCopperCmd](https://github.com/HiRoFa/GreenCopperCmd).

Please see the [DOCS](https://hirofa.github.io/quickjs_es_runtime/quickjs_runtime/index.html) for all inner workings

# This lib serves two main goals:

## 1. Provide simple utils for working with quickjs (these are located in the quickjs_utils mod)

* The QuickJsRuntime struct, this is to be used from a single thread
* E.g. objects::set_property(), functions::invoke_func()
* Wrap JSValue to provide reference counting (+1 on init, -1 on
  drop) ([QuickJsValueAdapter](https://hirofa.github.io/quickjs_es_runtime/quickjs_runtime/quickjsvalueadapter/struct.QuickJsValueAdapter.html))
* Pass a module loader

## 2. Wrap quickjs for use as a ready to go JavaScript Runtime

* Start at the QuickjsRuntimeFacade, it provides an EventQueue which has a thread_local QuickJsRuntimeAdapter
* All values are copied or abstracted in a JsValueFacades
* So no need to worry about Garbage collection
* Evaluate script and invoke functions while waiting for results blocking or with async/await
* Get Promise result blocking or with async/await

# What works?

## Script and Modules

* Typescript (via SWC)
* Console (
  .log/info/debug/trace/error) ([docs](https://hirofa.github.io/quickjs_es_runtime/quickjs_runtime/features/console/index.html))
* Eval
  script ([docs](https://hirofa.github.io/quickjs_es_runtime/quickjs_runtime/facades/struct.QuickJsRuntimeFacade.html#method.eval))
* Create promises in JavaScript which execute async
* Eval
  modules ([docs](https://hirofa.github.io/quickjs_es_runtime/quickjs_runtime/facades/struct.QuickJsRuntimeFacade.html#method.eval_module))
* Load modules (dynamic and
  static) ([docs](https://hirofa.github.io/quickjs_es_runtime/quickjs_runtime/builder/struct.QuickJsRuntimeBuilder.html#method.script_module_loader))
* Fetch api (impl in [GreenCopperRuntime](https://github.com/HiRoFa/GreenCopperRuntime))
* setImmediate
* setTimeout/Interval (and clear)
* Script preprocessing (impls for ifdef/macro's/typescript can be found
  in [GreenCopperRuntime](https://github.com/HiRoFa/GreenCopperRuntime))

## Rust-Script interoperability

* Return Promises from rust functions and resolve them from
  rust ([docs](https://hirofa.github.io/quickjs_es_runtime/quickjs_runtime/jsutils/promises/fn.new_resolving_promise.html))
* Add functions from
  rust ([docs](https://hirofa.github.io/quickjs_es_runtime/quickjs_runtime/quickjsrealmadapter/struct.QuickJsRealmAdapter.html#method.install_function))
* Invoke JS functions from
  rust ([docs](https://hirofa.github.io/quickjs_es_runtime/quickjs_runtime/facades/struct.QuickJsRuntimeFacade.html#method.invoke_function))
* Pass primitives, objects and arrays from and to
  rust ([docs](https://hirofa.github.io/quickjs_es_runtime/quickjs_runtime/quickjs_utils/primitives/index.html))
* Create Classes from
  rust ([docs](https://hirofa.github.io/quickjs_es_runtime/quickjs_runtime/reflection/struct.Proxy.html))
* Async/await support on eval/call_function/promise
  resolution ([docs](https://hirofa.github.io/quickjs_es_runtime/quickjs_runtime/values/struct.CachedJsPromiseRef.html#method.get_promise_result))
* Import native Modules (e.g. dynamic loading of rust functions or Proxy
  classes) ([docs](https://hirofa.github.io/quickjs_es_runtime/quickjs_runtime/builder/struct.QuickJsRuntimeBuilder.html#method.native_module_loader))

# Goals

Embedding a script engine in a rust project seems a very tedious job which involves learning a lot about the inner
workings of that engine.

The main goal of this project is to make that job easy!

The manner in which this is achieved is primarily focused on abstracting the workings of the engine from the
implementor, therefore some functionality may not be the fastest way of getting things done.

So a second goal is to make implementing a fast and efficient integration doable for the uninitiated, the most common
tasks you do with the engine should be doable with the utils in this package and working examples should be provided in
the test modules.

The reason I chose QuickJS as the engine is that I've been dealing with less modern engines in my java projects and not
being able to use the latest and greatest ECMA-script features becomes quite disappointing at times.

The fun stuff about QuickJS:

* small footprint
* fast compilation / startup
* great JS compatibility

# examples

Cargo.toml

```toml
[dependencies]
quickjs_runtime = "0.15"
```

Here are some quickstarts:

* start by reading the [DOCS](https://hirofa.github.io/quickjs_es_runtime/quickjs_runtime/index.html)
* [eval a script](https://hirofa.github.io/quickjs_es_runtime/quickjs_runtime/facades/struct.QuickJsRuntimeFacade.html#method.eval)

The quickjs Api utils:

* [quickjs_utils](https://hirofa.github.io/quickjs_es_runtime/quickjs_runtime/quickjs_utils/index.html)
