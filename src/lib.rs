//! # quickjs_runtime
//! This crate is made up of two main parts:
//! * thread-safe utils and wrappers
//!   you can call these from any thread, all logic is directed to a single worker-thread which talks to the quickjs API
//! * quickjs bindings and utils
//!   these talk to the quickjs API directly and need to run in the same thread as the Runtime
//!
//! | Thread safe | Runtime Thread-local |
//! | --- | --- |
//! | [EsRuntime](https://hirofa.github.io/quickjs_es_runtime/quickjs_runtime/esruntime/struct.EsRuntime.html) | [QuickJsRuntime](https://hirofa.github.io/quickjs_es_runtime/quickjs_runtime/quickjsruntime/struct.QuickJsRuntime.html) |
//! | [EsValueFacade](https://hirofa.github.io/quickjs_es_runtime/quickjs_runtime/esvalue/struct.EsValueFacade.html) | [JSValueRef](https://hirofa.github.io/quickjs_es_runtime/quickjs_runtime/valueref/struct.JSValueRef.html) |
//!
//! ## Doing something in the runtime worker thread
//!
//! You always start with building a new ESRuntime
//!
//! ```dontrun
//! use quickjs_runtime::esruntimebuilder::EsRuntimeBuilder;
//! let rt = EsRuntimeBuilder::new().build();
//! ```
//!
//! [EsRuntime](https://hirofa.github.io/quickjs_es_runtime/quickjs_runtime/esruntime/struct.EsRuntime.html) has plenty public methods you can check out but one the things you'l need to understand is how to communicate to the actual QuickJsRuntime
//! This is done by adding a job to the EventQueue of the EsRuntime
//!
//! ```dontrun
//! use quickjs_runtime::quickjsruntime::QuickJsRuntime;
//! rt.add_to_event_queue(|q_js_rt: &QuickJsRuntime| {
//!    // this will run in the Worker thread, here we can use the quickjs API
//! });
//! ```
//!
//! In order to do something and get the result you can use the sync variant
//! ```dontrun
//! use quickjs_runtime::quickjsruntime::QuickJsRuntime;
//! let res = rt.add_to_event_queue_sync(|q_js_rt: &QuickJsRuntime| {
//!    // this will run in the Worker thread, here we can use the quickjs API
//!    return 1;
//! });
//! ```
//!
//! For more details and examples please explore the packages below

#[macro_use]
extern crate lazy_static;

mod droppable_value;
pub mod eserror;
pub mod esruntime;
pub mod esruntime_utils;
pub mod esruntimebuilder;
pub mod esscript;
pub mod esvalue;
pub mod features;
pub mod quickjs_utils;
pub mod quickjscontext;
pub mod quickjsruntime;
pub mod reflection;
pub mod utils;
pub mod valueref;
