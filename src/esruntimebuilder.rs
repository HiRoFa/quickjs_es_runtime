use crate::eserror::EsError;
use crate::esruntime::{EsRuntime, FetchResponseProvider};
use crate::features::fetch::request::FetchRequest;
use crate::features::fetch::response::FetchResponse;
use crate::quickjsruntime::{NativeModuleLoader, QuickJsRuntime, ScriptModuleLoader};
use hirofa_utils::js_utils::ScriptPreProcessor;
use std::sync::Arc;
use std::time::Duration;

pub type EsRuntimeInitHooks =
    Vec<Box<dyn FnOnce(&EsRuntime) -> Result<(), EsError> + Send + 'static>>;

/// the EsRuntimeBuilder is used to init an EsRuntime
/// # Example
/// ```rust
/// use quickjs_runtime::esruntimebuilder::EsRuntimeBuilder;
/// // init a rt which may use 16MB of memory
/// let rt = EsRuntimeBuilder::new()
/// .memory_limit(1024*1024*16)
/// .build();
/// ```
pub struct EsRuntimeBuilder {
    pub(crate) script_module_loaders: Vec<Box<dyn ScriptModuleLoader + Send>>,
    pub(crate) native_module_loaders: Vec<Box<dyn NativeModuleLoader + Send>>,
    pub(crate) opt_fetch_response_provider: Option<Box<FetchResponseProvider>>,
    pub(crate) opt_memory_limit_bytes: Option<u64>,
    pub(crate) opt_gc_threshold: Option<u64>,
    pub(crate) opt_max_stack_size: Option<u64>,
    pub(crate) opt_gc_interval: Option<Duration>,
    pub(crate) runtime_init_hooks: EsRuntimeInitHooks,
    pub(crate) script_pre_processors: Vec<Box<dyn ScriptPreProcessor + Send>>,
    pub(crate) interrupt_handler: Option<Box<dyn Fn(&QuickJsRuntime) -> bool + Send>>,
}

impl EsRuntimeBuilder {
    /// build an EsRuntime
    pub fn build(self) -> Arc<EsRuntime> {
        EsRuntime::new(self)
    }

    /// init a new EsRuntimeBuilder
    pub fn new() -> Self {
        Self {
            script_module_loaders: vec![],
            native_module_loaders: vec![],
            opt_fetch_response_provider: None,
            opt_memory_limit_bytes: None,
            opt_gc_threshold: None,
            opt_max_stack_size: None,
            opt_gc_interval: None,
            runtime_init_hooks: vec![],
            script_pre_processors: vec![],
            interrupt_handler: None,
        }
    }

    /// add a script loaders which will be used to load modules when they are imported from script
    /// # Example
    /// ```rust
    /// use hirofa_utils::js_utils::Script;
    /// use quickjs_runtime::esruntimebuilder::EsRuntimeBuilder;
    /// use quickjs_runtime::quickjscontext::QuickJsContext;
    /// use quickjs_runtime::quickjsruntime::ScriptModuleLoader;
    /// struct MyModuleLoader {}
    /// impl ScriptModuleLoader for MyModuleLoader {
    ///     fn normalize_path(&self,ref_path: &str,path: &str) -> Option<String> {
    ///         Some(path.to_string())
    ///     }
    ///
    ///     fn load_module(&self, absolute_path: &str) -> String {
    ///         "export const foo = 12;".to_string()
    ///     }
    /// }
    ///
    /// let rt = EsRuntimeBuilder::new()
    ///     .script_module_loader(Box::new(MyModuleLoader{}))
    ///     .build();
    /// rt.eval_module_sync(Script::new("test_module.es", "import {foo} from 'some_module.mes';\nconsole.log('foo = %s', foo);")).ok().unwrap();
    /// ```
    pub fn script_module_loader<M: ScriptModuleLoader + Send + 'static>(
        mut self,
        loader: Box<M>,
    ) -> Self {
        self.script_module_loaders.push(loader);
        self
    }

    /// add a ScriptPreProcessor which will be called for all scripts which are evaluated and compiled
    pub fn script_pre_processor<S: ScriptPreProcessor + Send + 'static>(
        mut self,
        processor: S,
    ) -> Self {
        self.script_pre_processors.push(Box::new(processor));
        self
    }

    pub fn runtime_init_hook<H>(mut self, hook: H) -> Self
    where
        H: FnOnce(&EsRuntime) -> Result<(), EsError> + Send + 'static,
    {
        self.runtime_init_hooks.push(Box::new(hook));
        self
    }

    /// add a module loader which can load native functions and proxy classes
    /// # Example
    /// ```rust
    /// use quickjs_runtime::esruntimebuilder::EsRuntimeBuilder;
    /// use quickjs_runtime::quickjsruntime::NativeModuleLoader;
    /// use quickjs_runtime::valueref::JSValueRef;
    /// use quickjs_runtime::quickjscontext::QuickJsContext;
    /// use quickjs_runtime::quickjs_utils::functions;
    /// use quickjs_runtime::quickjs_utils::primitives::{from_bool, from_i32};
    /// use quickjs_runtime::reflection::Proxy;
    /// use hirofa_utils::js_utils::Script;
    ///
    /// struct MyModuleLoader{}
    /// impl NativeModuleLoader for MyModuleLoader {
    ///     fn has_module(&self, _q_ctx: &QuickJsContext,module_name: &str) -> bool {
    ///         module_name.eq("my_module")
    ///     }
    ///
    ///     fn get_module_export_names(&self, _q_ctx: &QuickJsContext, _module_name: &str) -> Vec<&str> {
    ///         vec!["someVal", "someFunc", "SomeClass"]
    ///     }
    ///
    ///     fn get_module_exports(&self, q_ctx: &QuickJsContext, _module_name: &str) -> Vec<(&str, JSValueRef)> {
    ///         
    ///         let js_val = from_i32(1470);
    ///         let js_func = functions::new_function_q(
    ///             q_ctx,
    ///             "someFunc", |_q_ctx, _this, _args| {
    ///                 return Ok(from_i32(432));
    ///             }, 0)
    ///             .ok().unwrap();
    ///         let js_class = Proxy::new()
    ///             .name("SomeClass")
    ///             .static_method("doIt", |_q_ctx, _args|{
    ///                 return Ok(from_i32(185));
    ///             })
    ///             .install(q_ctx, false)
    ///             .ok().unwrap();
    ///
    ///         vec![("someVal", js_val), ("someFunc", js_func), ("SomeClass", js_class)]
    ///     }
    /// }
    ///
    /// let rt = EsRuntimeBuilder::new()
    /// .native_module_loader(Box::new(MyModuleLoader{}))
    /// .build();
    ///
    /// rt.eval_module_sync(Script::new("test_native_mod.es", "import {someVal, someFunc, SomeClass} from 'my_module';\nlet i = (someVal + someFunc() + SomeClass.doIt());\nif (i !== 2087){throw Error('i was not 2087');}")).ok().expect("script failed");
    /// ```
    pub fn native_module_loader<M: NativeModuleLoader + Send + 'static>(
        mut self,
        loader: Box<M>,
    ) -> Self {
        self.native_module_loaders.push(loader);
        self
    }

    /// Provide a fetch response provider in order to make the fetch api work in the EsRuntime
    /// # Example
    /// ```rust
    ///
    /// use quickjs_runtime::esruntimebuilder::EsRuntimeBuilder;
    /// use quickjs_runtime::features::fetch::response::FetchResponse;
    /// use quickjs_runtime::features::fetch::request::FetchRequest;
    /// use hirofa_utils::js_utils::Script;
    /// use std::time::Duration;   
    ///
    /// struct SimpleResponse{
    ///     read_done: bool
    /// }
    ///
    /// impl SimpleResponse {
    ///     fn new(_req: &FetchRequest) -> Self {
    ///         Self{read_done:false}
    ///     }
    /// }
    ///
    /// impl FetchResponse for SimpleResponse {
    ///     fn get_http_status(&self) -> u16 {
    ///         200
    ///     }
    ///
    ///     fn get_header(&self,name: &str) -> Option<&str> {
    ///         unimplemented!()
    ///     }
    ///
    ///     fn read(&mut self) -> Option<Vec<u8>> {
    ///         if self.read_done {
    ///             None
    ///         } else {
    ///             self.read_done = true;      
    ///             Some("Hello world".as_bytes().to_vec())
    ///         }
    ///     }
    /// }
    ///
    /// let rt = EsRuntimeBuilder::new()
    /// .fetch_response_provider(|req| {Box::new(SimpleResponse::new(req))})
    /// .build();
    ///
    /// let res_prom = rt.eval_sync(Script::new("test_fetch.es", "(fetch('something')).then((fetchRes) => {return fetchRes.text();});")).ok().expect("script failed");
    /// let res = res_prom.get_promise_result_sync();
    /// let str_esvf = res.ok().expect("promise did not resolve ok");
    /// assert_eq!(str_esvf.get_str(), "Hello world");
    /// ```
    pub fn fetch_response_provider<P>(mut self, provider: P) -> Self
    where
        P: Fn(&FetchRequest) -> Box<dyn FetchResponse + Send> + Send + Sync + 'static,
    {
        assert!(self.opt_fetch_response_provider.is_none());
        self.opt_fetch_response_provider = Some(Box::new(provider));
        self
    }

    /// set max memory the runtime may use
    pub fn memory_limit(mut self, bytes: u64) -> Self {
        self.opt_memory_limit_bytes = Some(bytes);
        self
    }

    /// number of allocations before gc is run
    pub fn gc_threshold(mut self, size: u64) -> Self {
        self.opt_gc_threshold = Some(size);
        self
    }

    /// set a max stack size
    pub fn max_stack_size(mut self, size: u64) -> Self {
        self.opt_max_stack_size = Some(size);
        self
    }

    /// set a Garbage Collection interval, this will start a timer thread which will trigger a full GC every set interval
    pub fn gc_interval(mut self, interval: Duration) -> Self {
        self.opt_gc_interval = Some(interval);
        self
    }

    /// add an interrupt handler, this will be called several times during script execution and may be used to cancel a running script
    pub fn set_interrupt_handler<I: Fn(&QuickJsRuntime) -> bool + Send + 'static>(
        &mut self,
        interrupt_handler: I,
    ) -> &mut Self {
        self.interrupt_handler = Some(Box::new(interrupt_handler));
        self
    }
}

impl Default for EsRuntimeBuilder {
    fn default() -> Self {
        EsRuntimeBuilder::new()
    }
}

#[cfg(test)]
pub mod tests {
    use crate::esruntimebuilder::EsRuntimeBuilder;
    use crate::quickjsruntime::ScriptModuleLoader;
    use hirofa_utils::js_utils::Script;

    #[test]
    fn test_module_loader() {
        struct MyModuleLoader {}
        impl ScriptModuleLoader for MyModuleLoader {
            fn normalize_path(&self, _ref_path: &str, path: &str) -> Option<String> {
                Some(path.to_string())
            }

            fn load_module(&self, _absolute_path: &str) -> String {
                "export const foo = 12;".to_string()
            }
        }

        let rt = EsRuntimeBuilder::new()
            .script_module_loader(Box::new(MyModuleLoader {}))
            .build();
        match rt.eval_module_sync(Script::new(
            "test_module.es",
            "import {foo} from 'some_module.mes';\nconsole.log('foo = %s', foo);",
        )) {
            Ok(_) => {}
            Err(e) => panic!("script failed {}", e),
        }
    }
}
