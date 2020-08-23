use crate::eseventqueue::EsEventQueue;
use crate::esruntimebuilder::EsRuntimeBuilder;
use crate::esscript::EsScript;
use crate::quickjsruntime::QuickJsRuntime;
use quick_js::{ExecutionError, JsValue};
use std::sync::Arc;

pub struct EsRuntime {
    event_queue: Arc<EsEventQueue>,
}

impl EsRuntime {
    pub(crate) fn new(_builder: EsRuntimeBuilder) -> Arc<Self> {
        Arc::new(Self {
            event_queue: EsEventQueue::new(),
        })
    }

    pub fn builder() -> EsRuntimeBuilder {
        EsRuntimeBuilder::new()
    }

    pub fn eval(&self, script: EsScript) {
        self.add_to_event_queue(|qjs_rt| {
            let res = qjs_rt.eval(script);
            match res {
                Ok(_) => {}
                Err(e) => log::error!("error in async eval {}", e),
            }
        });
    }

    pub fn eval_sync(&self, script: EsScript) -> Result<JsValue, ExecutionError> {
        self.add_to_event_queue_sync(|qjs_rt| qjs_rt.eval(script))
    }

    pub fn gc() {}

    pub fn gc_sync() {}

    pub fn load_module() {}

    pub fn load_module_sync() {}

    pub fn new_class_builder() {}

    pub fn add_to_event_queue<C>(&self, consumer: C)
    where
        C: FnOnce(&QuickJsRuntime) + Send + 'static,
    {
        self.event_queue
            .add_task(|| QuickJsRuntime::do_with(consumer));
    }

    pub fn add_to_event_queue_sync<C, R>(&self, consumer: C) -> R
    where
        C: FnOnce(&QuickJsRuntime) -> R + Send + 'static,
        R: Send + 'static,
    {
        self.event_queue
            .exe_task(|| QuickJsRuntime::do_with(consumer))
    }

    pub fn add_helper_task() {}
}

#[cfg(test)]
pub mod tests {
    use crate::esruntime::EsRuntime;
    use crate::esscript::EsScript;
    use log::LevelFilter;
    use quick_js::JsValue;
    use std::sync::Arc;

    lazy_static! {
        pub static ref TEST_ESRT: Arc<EsRuntime> = init();
    }

    fn init() -> Arc<EsRuntime> {
        simple_logging::log_to_file("esruntime.log", LevelFilter::max())
            .ok()
            .expect("could not init logger");
        EsRuntime::builder().build()
    }

    #[test]
    fn test_eval_sync() {
        let rt = &TEST_ESRT;
        rt.eval(EsScript::new(
            "test.es".to_string(),
            "console.log('foo bar');".to_string(),
        ));

        let res = rt
            .eval_sync(EsScript::new("test.es".to_string(), "(2 * 7);".to_string()))
            .ok()
            .expect("script failed");

        assert_eq!(res, JsValue::Int(14));
    }
}
