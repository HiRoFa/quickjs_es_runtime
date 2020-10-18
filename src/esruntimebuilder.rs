use crate::esruntime::{EsRuntime, FetchResponseProvider};
use crate::esscript::EsScript;
use crate::features::fetch::request::FetchRequest;
use crate::features::fetch::response::FetchResponse;
use crate::quickjsruntime::ModuleScriptLoader;
use std::sync::Arc;
use std::time::Duration;

/// the EsRuntimeBuilder is used to init an EsRuntime
/// # Example
/// ```rust
/// use quickjs_es_runtime::esruntimebuilder::EsRuntimeBuilder;
/// // init a rt which may use 16MB of memory
/// let rt = EsRuntimeBuilder::new()
/// .memory_limit(1024*1024*16)
/// .build();
/// ```
pub struct EsRuntimeBuilder {
    pub(crate) opt_module_script_loader: Option<Box<ModuleScriptLoader>>,
    pub(crate) opt_fetch_response_provider: Option<Box<FetchResponseProvider>>,
    pub(crate) opt_memory_limit_bytes: Option<u64>,
    pub(crate) opt_gc_threshold: Option<u64>,
    pub(crate) opt_max_stack_size: Option<u64>,
    pub(crate) opt_gc_interval: Option<Duration>,
}

impl EsRuntimeBuilder {
    /// build an EsRuntime
    pub fn build(self) -> Arc<EsRuntime> {
        EsRuntime::new(self)
    }

    /// init a new EsRuntimeBuilder
    pub fn new() -> Self {
        Self {
            opt_module_script_loader: None,
            opt_fetch_response_provider: None,
            opt_memory_limit_bytes: None,
            opt_gc_threshold: None,
            opt_max_stack_size: None,
            opt_gc_interval: None,
        }
    }

    /// add a script loaders which will be used to load modules when they are imported from script
    /// # Example
    /// ```rust
    /// use quickjs_es_runtime::esscript::EsScript;
    ///     use quickjs_es_runtime::esruntimebuilder::EsRuntimeBuilder;
    /// fn load_module(base: &str, name: &str) -> Option<EsScript> {
    ///     // you should load your modules from files here
    ///     // please note that you need to return the name as absolute_path in the returned script struct
    ///     // return None if module is not found
    ///     Some(EsScript::new(name, "export const foo = 12;"))
    /// }
    /// fn main(){
    ///     let rt = EsRuntimeBuilder::new()
    ///         .module_script_loader(load_module)
    ///         .build();
    /// }
    /// ```
    pub fn module_script_loader<M>(mut self, loader: M) -> Self
    where
        M: Fn(&str, &str) -> Option<EsScript> + Send + Sync + 'static,
    {
        self.opt_module_script_loader = Some(Box::new(loader));
        self
    }

    pub fn fetch_response_provider<P>(mut self, provider: P) -> Self
    where
        P: Fn(&FetchRequest) -> Box<dyn FetchResponse + Send> + Send + Sync + 'static,
    {
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

    pub fn gc_interval(mut self, interval: Duration) -> Self {
        self.opt_gc_interval = Some(interval);
        self
    }
}

impl Default for EsRuntimeBuilder {
    fn default() -> Self {
        EsRuntimeBuilder::new()
    }
}
