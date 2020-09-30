use crate::esruntime::EsRuntime;
use crate::esscript::EsScript;
use crate::quickjsruntime::ModuleScriptLoader;
use std::sync::Arc;
use std::time::Duration;

// todo
// JS_SetMemoryLimit
// JS_SetGCThreshold
// JS_SetMaxStackSize

pub struct EsRuntimeBuilder {
    pub(crate) loader: Option<Box<ModuleScriptLoader>>,
    pub(crate) memory_limit_bytes: Option<usize>,
    pub(crate) _gc_interval: Option<Duration>,
    pub(crate) _helper_thread_count: Option<usize>,
}

impl EsRuntimeBuilder {
    pub fn build(self) -> Arc<EsRuntime> {
        EsRuntime::new(self)
    }
    pub fn new() -> Self {
        Self {
            loader: None,
            memory_limit_bytes: None,
            _gc_interval: None,
            _helper_thread_count: None,
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
        self.loader = Some(Box::new(loader));
        self
    }

    /// maximate the memory the runtime may use
    pub fn memory_limit<M>(mut self, bytes: usize) -> Self {
        self.memory_limit_bytes = Some(bytes);
        self
    }
}

impl Default for EsRuntimeBuilder {
    fn default() -> Self {
        EsRuntimeBuilder::new()
    }
}
