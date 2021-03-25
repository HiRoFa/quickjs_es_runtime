# 0.2.3 (work in progress)

* added a runtime_init_hook(hook) method to the EsRuntimeBuilder so we can add vars to the runtime when the builder is built
* added EsPromise::new_async which can be used to instantiate a Promise with an async resolver
* refactored the module loader sin qjsrt so we can differentiate between sacript and native.. i need that for CommonJS and probably later for transpiling and such

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