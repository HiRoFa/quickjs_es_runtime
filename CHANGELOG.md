# 0.10.2

* removed error when dropping rt/ctx (finalizers of proxy classes causing an allreadyborrowed panic )
* Proxy instances now have a .constructor which is the constructor function..

# 0.10.1

* replaced Mutex with DebugMutex and thus with parking_lot

# 0.10.0

* removed Js*Adapter/Facade traits
* renamed JSValueRef to QuickjsValueAdapter
* removed legacy EsValueFacade
* removed Weak<QuickjsRuntimeFacadeInner> ref arg from JsValueFacade inners
* renamed all js_ functions to more readable names (eg js_null_create()) -> create_null())
* added (static_)catch_all_getter_setter to Proxy for getting/setting all prop names
* altered the way things are parsed in reflection, like propnames.. should lead to less string allocation

# 0.9.0

* when dropping a Realm dangling Promises will log an error instead of panicking

# 0.8

## 0.8.7

* removed win-api and once_cell from tokio features
* serde support in utils

## 0.8.6

* turned obj is proxy code around to prevent errors filling the log

## 0.8.5

* better toString for errors

## 0.8.4

* removed some logging

## 0.8.3

* reference utils 0.5.4 (by reffing 0.5) (fixes #69)

## 0.8.2

* reference utils 0.5.3 (fixes #68)

## 0.8.1

* fixed memory usage report (#66)

## 0.8.0

* impld realm init hook
* proxy info functions (see if obj is an instance of a proxy class)

# 0.7

## 0.7.2 

* fix for #62 (nested callback creation/drop fails)

## 0.7.1 

* fn to calc memory usage
* implemented static event handlers for proxies

## 0.7.0

* implemented js_proxy_new_instance_with_id from utils which allows you to create an instance of a proxy with a predefined id
* implemented set_prop / has_prop functions in proxies
* implemented CompiledModuleLoader
* implemented jsValueAdapter.js_to_str()
* changed some deps to minor version x.x instead of x.x.x
* made console/setinterval/settimeout/setimmediate optional(but default) features
* more complete stacktrace with errors
* callback functions add name to stacktrace on error
* implemented typedarrays (Uint8 only for now)

# 0.6

## 0.6.0

* updated quickjs to 2021-03-27
* reverted back to EsRuntime having an Arc<EsRuntimeInner> (helps me with my abstraction project)
* renamed EsRuntime to QuickJsRuntimeFacade, and others to follow same conventions
* removed fetch api (moved to greencopperruntime)
* removed all panics when async promise resolution fails because of the realm being invalid
* implemented a lot of js_utils abstractions, please note that js_utils::JsValueFacade will someday deprecate quickjs_runtime::EsValueFacade

# 0.5

## 0.5.1

* removed redundant prinltn (thanks SreeniIO!)
* added testcase for abstractions, and fixed some typedefs

## 0.5.0

* replaced EsScript with js_utils::Script
* replaced ScriptPreProcessor with js_utils::ScriptPreProcessor
* replaced EsError with js_utils::JsError
* implemented utils::js_utils (generic adapters and facades, see [the green copper plan](https://github.com/HiRoFa/GreenCopperRuntime/blob/main/README.md#roadmap--the-plan) for what's this all about)
* changes to function definitions
* fixed interrupt handler 

# 0.4

## 0.4.2

* moved reflection code to reflection/mod.rs (should not affect api)
* toPrimitive for Proxy classes (do stuff like console.log('got: ' + MyProxyInstanceOrClass))
* removed droppablevalue, replaced with JSPropertyEnumRef
* added is_enumerable(index) fn to JSPropertyEnumRef  
* added get_name(index) fn to JSPropertyEnumRef
* added interrupt_handler

## 0.4.1 

* altered tokio dep, full is now optional (prevents valgrind errors)
* altered utils dep to 0.1
* added debug log for when eval/evalmodule fails
* scriptpreproc returns Result instead of just script
* q_js_rt.load_module_script_opt now returns Script instead of String

## 0.4.0

* use EventLoop from hirofa_utils, cleaner code, much less Mutexes
* Renamed a lot of public methods
  * e.g. rt.add_to_es_event_queue_sync -> rt.exe_rt_task_in_event_loop()
  * e.g. rt.add_to_es_event_queue -> rt.add_rt_task_to_event_loop()
* Removed EsRuntime.inner Arc, was a duplicate solution to the same problem
* mit lic
* script preprocessors

# 0.3.0

* EsValueFacade now links to live object when object is passed out of runtime
* added EsValueFacade.stringify()
* added EsFunction struct to create functions as EsValueFacade
* is/get_error for EsValueFacade used for when promise or async function is rejected with Error obj

# 0.2

## 0.2.3 

* added a runtime_init_hook(hook) method to the EsRuntimeBuilder so we can add vars to the runtime when the builder is built
* added EsPromise::new_async which can be used to instantiate a Promise with an async resolver
  * Big thanks to [SreeniIO](https://github.com/SreeniIO) for helping out!
* refactored the module loaders in qjsrt so we can differentiate between script and native... I need that for CommonJS and probably later for transpiling and such
* added quickjs_utils::get_script_or_module_name to get the current scripts filename or module name
* Proxy supports multiple finalizers
* Proxy as EventTarget first working code. needs to mature, but the goal for now is minimal support for dispatching events from rust to JavaScript 

## 0.2.2

* added _void variants for adding jobs to the event queue, this prevents Futures being dropped before being resolved resulting in errors in the logs
* setTimeout / setInterval now correctly run pending jobs (fixes resolving promises with timeout and such)
* removed logging from console.rs so we can set custom loglevel to that package
* fixed deadlocks in esvalue promise resolution

## 0.2.1

* altered esruntimebuilder to accept Box<M>

## 0.2.0

* rebuilt the module loading system
* use Waker in Futures

# 0.1

## 0.1.1

* more precise timing for setTimeout and setInterval
* quickjs_utils::maps utils for handling [Map](https://developer.mozilla.org/en-US/docs/Web/JavaScript/Reference/Global_Objects/Map) instances from rust
* quickjs_utils::sets utils for handling [Set](https://developer.mozilla.org/en-US/docs/Web/JavaScript/Reference/Global_Objects/Set) instances from rust
* Proxy.event_target and Proxy.static_event_target to allow a Proxy to be used as [EventTarget](https://developer.mozilla.org/en-US/docs/Web/API/EventTarget)
* objects::construct_object util to create a new instance of a constructor
* iterators util to handle iterators
* EventQueue.async_task will be a starting point for being able to use async/await with javascript
* made async functions in EsRuntime (eval, eval_module, call_function, gc)
* made invoke_function in EsValueFacade async
* added async [get_promise_result](https://hirofa.github.io/quickjs_es_runtime/quickjs_runtime/esvalue/struct.EsValueFacade.html#method.get_promise_result) to EsValueFacade so the result of a promise may be awaited async
* added quickjs_utils::modules::detect_module() method to detect if a script source is a module
* added ```es_args![]``` macro so you can use
  
  ```let args = es_args![1, 2, true, "sdf".to_string()]```
  
  instead of
  
  ```let args = vec![1.to_es_value_facade(), 2.to_es_value_facade(), true.to_es_value_facade(), "sdf".to_string().to_es_value_facade()]```
* added quickjs_utils::modules::new_module/add_module_export/set_module_export
* added NativeModuleLoader to QuickJSRuntime to enable implementors to load native modules on-demand
* altered reflection to enable creation of JSValueRef without making it available in global scope (.install(ctx, false))

## 0.1.0

Initial release
