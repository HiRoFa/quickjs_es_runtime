use crate::esruntime::EsRuntime;
use std::sync::Arc;

pub struct EsRuntimeBuilder {}

impl EsRuntimeBuilder {
    pub fn build(self) -> Arc<EsRuntime> {
        EsRuntime::new(self)
    }
    pub fn new() -> Self {
        Self {}
    }
}
