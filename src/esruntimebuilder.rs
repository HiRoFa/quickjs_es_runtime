use crate::esruntime::EsRuntime;
use crate::esscript::EsScript;
use crate::quickjsruntime::ModuleScriptLoader;
use std::sync::Arc;
use std::time::Duration;

pub struct EsRuntimeBuilder {
    pub(crate) loader: Option<Box<ModuleScriptLoader>>,
    pub(crate) _memory_limit_mb: Option<usize>,
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
            _memory_limit_mb: None,
            _gc_interval: None,
            _helper_thread_count: None,
        }
    }

    pub fn module_script_loader<M>(&mut self, loader: M) -> &mut Self
    where
        M: Fn(&str, &str) -> Option<EsScript> + Send + Sync + 'static,
    {
        self.loader = Some(Box::new(loader));
        self
    }
}

impl Default for EsRuntimeBuilder {
    fn default() -> Self {
        EsRuntimeBuilder::new()
    }
}
