use crate::eserror::EsError;
use crate::eseventqueue::EsEventQueue;
use crate::esruntimebuilder::EsRuntimeBuilder;
use crate::esscript::EsScript;
use crate::esvalue::EsValueFacade;
use crate::features;
use crate::quickjsruntime::QuickJsRuntime;
use log::error;
use std::sync::Arc;

pub struct EsRuntime {
    event_queue: Arc<EsEventQueue>,
}

impl EsRuntime {
    pub(crate) fn new(_builder: EsRuntimeBuilder) -> Arc<Self> {
        let ret = Arc::new(Self {
            event_queue: EsEventQueue::new(),
        });

        let res = ret.add_to_event_queue_sync(|q_js_rt| features::init(q_js_rt));
        if res.is_err() {
            panic!("could not init features: {}", res.err().unwrap());
        }

        ret
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

    pub fn eval_sync(&self, script: EsScript) -> Result<EsValueFacade, EsError> {
        self.add_to_event_queue_sync(|qjs_rt| {
            let res = qjs_rt.eval(script);
            match res {
                Ok(val_ref) => EsValueFacade::from_jsval(qjs_rt, &val_ref),
                Err(e) => Err(e),
            }
        })
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

    pub fn eval_module_sync(&self, script: EsScript) -> Result<EsValueFacade, EsError> {
        self.add_to_event_queue_sync(|qjs_rt| {
            let res = qjs_rt.eval_module(script);
            match res {
                Ok(val_ref) => EsValueFacade::from_jsval(qjs_rt, &val_ref),
                Err(e) => Err(e),
            }
        })
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
        let res = self
            .event_queue
            .exe_task(|| QuickJsRuntime::do_with(consumer));
        self._add_job_run_task();
        res
    }

    pub fn add_helper_task() {}

    fn _add_job_run_task(&self) {
        log::trace!("EsRuntime._add_job_run_task!");
        self.event_queue.add_task(|| {
            QuickJsRuntime::do_with(|quick_js_rt| {
                log::trace!("EsRuntime._add_job_run_task > async!");
                while quick_js_rt.has_pending_jobs() {
                    log::trace!("quick_js_rt.has_pending_jobs!");
                    let res = quick_js_rt.run_pending_job();
                    match res {
                        Ok(_) => {
                            log::trace!("run_pending_job OK!");
                        }
                        Err(e) => {
                            error!("run_pending_job failed: {}", e);
                        }
                    }
                }
            })
        });
    }
}

#[cfg(test)]
pub mod tests {
    use crate::eserror::EsError;
    use crate::esruntime::EsRuntime;
    use crate::esscript::EsScript;
    use crate::esvalue::EsValueFacade;
    use ::log::debug;
    use log::LevelFilter;
    use std::sync::Arc;
    use std::time::Duration;

    lazy_static! {
        pub static ref TEST_ESRT: Arc<EsRuntime> = init();
    }

    fn init() -> Arc<EsRuntime> {
        log::trace!("TEST_ESRT::init");
        simple_logging::log_to_file("esruntime.log", LevelFilter::max())
            .ok()
            .expect("could not init logger");
        EsRuntime::builder().build()
    }

    #[test]
    fn test_eval_sync() {
        let rt: Arc<EsRuntime> = TEST_ESRT.clone();
        let res = rt.eval_sync(EsScript::new(
            "test.es".to_string(),
            "console.log('foo bar');".to_string(),
        ));

        match res {
            Ok(_) => {}
            Err(e) => {
                panic!("eval failed: {}", e);
            }
        }

        let res = rt
            .eval_sync(EsScript::new("test.es".to_string(), "(2 * 7);".to_string()))
            .ok()
            .expect("script failed");

        assert_eq!(res.get_i32(), 14);
    }

    #[test]
    fn test_promise() {
        let rt: Arc<EsRuntime> = TEST_ESRT.clone();

        let res = rt.eval_sync(EsScript::new(
            "testp2.es".to_string(),
            "let test_promise_P = (new Promise(function(res, rej) {console.log('before res');res(123);console.log('after res');}).then(function (a) {console.log('prom ressed to ' + a);}).catch(function(x) {console.log('p.ca ex=' + x);}))".to_string(),
        ));

        match res {
            Ok(_) => {}
            Err(e) => panic!("p script failed: {}", e),
        }
        std::thread::sleep(Duration::from_secs(1));
    }

    #[test]
    fn test_module_sync() {
        let rt = &TEST_ESRT;
        debug!("test static import");
        let res: Result<EsValueFacade, EsError> = rt.eval_module_sync(EsScript::new(
            "test.es".to_string(),
            "import {some} from 'test_module.mes';\n console.log(some.foo);".to_string(),
        ));

        match res {
            Ok(_) => {}
            Err(e) => {
                log::error!("static import failed: {}", e);
            }
        }

        debug!("test dynamic import");
        let res: Result<EsValueFacade, EsError> = rt.eval_module_sync(EsScript::new(
            "test.es".to_string(),
            "console.log('about to load dynamic module');import('test_module.mes').then((some) => {console.log('after dyn ' + some);console.log(some.mltpl(1, 2));}).catch((x) => {console.log('imp.cat x=' + x);});".to_string(),
        ));

        match res {
            Ok(_) => {}
            Err(e) => {
                log::error!("dynamic import failed: {}", e);
            }
        }
    }
}
