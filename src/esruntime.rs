use crate::eseventqueue::EsEventQueue;
use crate::esruntimebuilder::EsRuntimeBuilder;
use crate::esscript::EsScript;
use crate::quickjsruntime::QuickJsRuntime;
use quick_js::{ExecutionError, JsValue};
use std::sync::Arc;
use log::error;

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

    pub fn eval_module(&self, script: EsScript) {
        self.add_to_event_queue(|qjs_rt| {
            let res = qjs_rt.eval_module(script);
            match res {
                Ok(_) => {}
                Err(e) => log::error!("error in async eval {}", e),
            }
        });
    }

    pub fn eval_module_sync(&self, script: EsScript) -> Result<JsValue, ExecutionError> {
        self.add_to_event_queue_sync(|qjs_rt| qjs_rt.eval_module(script))
    }

    pub fn new_class_builder() {}

    pub fn add_to_event_queue<C>(&self, consumer: C)
    where
        C: FnOnce(&QuickJsRuntime) + Send + 'static,
    {
        self.event_queue
            .add_task(|| QuickJsRuntime::do_with(consumer));
        self._add_job_run_task();
    }

    pub fn add_to_event_queue_sync<C, R>(&self, consumer: C) -> R
    where
        C: FnOnce(&QuickJsRuntime) -> R + Send + 'static,
        R: Send + 'static,
    {
        let res = self.event_queue
            .exe_task(|| QuickJsRuntime::do_with(consumer));
        self._add_job_run_task();
        res
    }

    pub fn add_helper_task() {}

    fn _add_job_run_task(&self) {
        self.event_queue.add_task(|| {
            QuickJsRuntime::do_with(|quick_js_rt| {
                while quick_js_rt.has_pending_jobs() {

                    let res = quick_js_rt.run_pending_job();
                    match res {
                        Ok(_) => {},
                        Err(e) => {
                            error!("job run failed: {}", e);
                        },
                    }

                }
            })
        });
    }
}

#[cfg(test)]
pub mod tests {
    use crate::esruntime::EsRuntime;
    use crate::esscript::EsScript;
    use log::LevelFilter;
    use quick_js::{JsValue, ExecutionError};
    use std::sync::Arc;
    use::log::debug;

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
        rt.eval_sync(EsScript::new(
            "test.es".to_string(),
            "console.log('foo bar');".to_string(),
        )).ok().expect("eval script failed");



        let res = rt
            .eval_sync(EsScript::new("test.es".to_string(), "(2 * 7);".to_string()))
            .ok()
            .expect("script failed");

        assert_eq!(res, JsValue::Int(14));
    }

    #[test]
    fn test_promise(){
        let rt = &TEST_ESRT;

        rt.eval_sync(EsScript::new(
            "testp2.es".to_string(),
            "let r = {a: 1};console.log('setting up prom');let p = new Promise((res, rej) => {console.log('before res');res(123);console.log('after res');return 456;}).then((a) => {r.a = 2;console.log('prom ressed to ' + a);}).catch((x) => {console.log('p.ca ex=' + x);});".to_string(),
        )).ok().expect("eval script failed");

        rt.eval_sync(EsScript::new(
            "testp2.es".to_string(),
            "console.log('r.a = ' + r.a + ' p= ' + p);".to_string(),
        )).ok().expect("eval script failed");
    }

    #[test]
    fn test_module_sync() {
        let rt = &TEST_ESRT;
        debug!("test static import");
        let res: Result<JsValue, ExecutionError> = rt.eval_module_sync(EsScript::new(
            "test.es".to_string(),
            "import {some} from 'test_module.mes';\n console.log(some.foo);".to_string(),
        ));

        match res {
            Ok(_) => {},
            Err(e) => {
                log::error!("static import failed: {}", e);
            },
        }

        debug!("test dynamic import");
        let res: Result<JsValue, ExecutionError> = rt.eval_module_sync(EsScript::new(
            "test.es".to_string(),
            "console.log('about to load dynamic module');import('test_module.mes').then((some) => {console.log('after dyn ' + some);console.log(some.mltpl(1, 2));}).catch((x) => {console.log('imp.cat x=' + x);});".to_string(),
        ));

        match res {
            Ok(_) => {},
            Err(e) => {
                log::error!("dynamic import failed: {}", e);
            },
        }

    }
}
