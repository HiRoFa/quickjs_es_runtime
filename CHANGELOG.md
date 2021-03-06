# 0.5 (work in progress)

* replaced EsScript with js_utils::Script
* replaced ScriptPreProcessor with js_utils::ScriptPreProcessor
* replaced EsError with js_utils::JsError
* implemented utils::js_utils (generic adapters and facades, see [the green copper plan](https://github.com/HiRoFa/GreenCopperRuntime/blob/main/README.md#roadmap--the-plan) for what's this all about)
* changes to function definitions
* fixed interrupt handler 

# 0.4.2

* moved reflection code to reflection/mod.rs (should not affect api)
* toPrimitive for Proxy classes (do stuff like console.log('got: ' + MyProxyInstanceOrClass))
* removed droppablevalue, replaced with JSPropertyEnumRef
* added is_enumerable(index) fn to JSPropertyEnumRef  
* added get_name(index) fn to JSPropertyEnumRef
* added interrupt_handler

# 0.4.1 

* altered tokio dep, full is now optional (prevents valgrind errors)
* altered utils dep to 0.1
* added debug log for when eval/evalmodule fails
* scriptpreproc returns Result instead of just script
* q_js_rt.load_module_script_opt now returns Script instead of String

# 0.4.0

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

# 0.2.3 

* added a runtime_init_hook(hook) method to the EsRuntimeBuilder so we can add vars to the runtime when the builder is built
* added EsPromise::new_async which can be used to instantiate a Promise with an async resolver
  * Big thanks to [SreeniIO](https://github.com/SreeniIO) for helping out!
* refactored the module loaders in qjsrt so we can differentiate between script and native... I need that for CommonJS and probably later for transpiling and such
* added quickjs_utils::get_script_or_module_name to get the current scripts filename or module name
* Proxy supports multiple finalizers
* Proxy as EventTarget first working code. needs to mature, but the goal for now is minimal support for dispatching events from rust to JavaScript 

# 0.2.2

* added _void variants for adding jobs to the event queue, this prevents Futures being dropped before being resolved resulting in errors in the logs
* setTimeout / setInterval now correctly run pending jobs (fixes resolving promises with timeout and such)
* removed logging from console.rs so we can set custom loglevel to that package
* fixed deadlocks in esvalue promise resolution

# 0.2.1

* altered esruntimebuilder to accept Box<M>

# 0.2.0

* rebuilt the module loading system
* use Waker in Futures

# 0.1.1

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

# 0.1.0

Initial release
