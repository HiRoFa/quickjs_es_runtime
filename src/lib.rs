//! # quickjs_runtime
//! This crate is made up of two main parts:
//! * thread-safe utils and wrappers
//!   you can call these from any thread, all logic is directed to a single worker-thread which talks to the quickjs API
//! * quickjs bindings and utils
//!   these talk to the quickjs API directly and need to run in the same thread as the Runtime
//!
//! ## Noteworthy structs
//!
//! These are the structs you'll use the most
//!
//! | Thread safe | Runtime Thread-local |
//! | --- | --- |
//! | [EsRuntime](esruntime/struct.EsRuntime.html) the 'starting point' | [QuickJsRuntime](quickjsruntime/struct.QuickJsRuntime.html) the wrapper for all things quickjs |
//! | [EsValueFacade](esvalue/struct.EsValueFacade.html) copy of or reference to a value in the Runtime | [JSValueRef](valueref/struct.JSValueRef.html) reference counting pointer to a Value |
//!
//! ## Doing something in the runtime worker thread
//!
//! You always start with building a new [EsRuntime](esruntime/struct.EsRuntime.html)
//!
//! ```dontrun
//! use quickjs_runtime::esruntimebuilder::EsRuntimeBuilder;
//! let rt: EsRuntime = EsRuntimeBuilder::new().build();
//! ```
//!
//! [EsRuntime](https://hirofa.github.io/quickjs_es_runtime/quickjs_runtime/esruntime/struct.EsRuntime.html) has plenty public methods you can check out but one of the things you'll need to understand is how to communicate with the QuickJsRuntime
//! This is done by adding a job to the [EventQueue](utils/single_threaded_event_queue/struct.SingleThreadedEventQueue.html) of the [EsRuntime](esruntime/struct.EsRuntime.html)
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

#[macro_export]
macro_rules! es_args {
    ( $( $x:expr ),* ) => {
        {
            let mut temp_vec: Vec<crate::esvalue::EsValueFacade> = Vec::new();
            $(
                temp_vec.push(crate::esvalue::EsValueConvertible::to_es_value_facade($x));
            )*
            temp_vec
        }
    };
}

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

#[cfg(test)]
pub mod tests {

    #[test]
    fn test_macro() {
        let _args = es_args!(1, 2i32, true, "sdf".to_string());
    }
}
