# 0.1.1 (Work in progress)

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

# 0.1.0

Initial release