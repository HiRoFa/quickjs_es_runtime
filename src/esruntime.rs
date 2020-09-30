use crate::eserror::EsError;
use crate::esruntimebuilder::EsRuntimeBuilder;
use crate::esscript::EsScript;
use crate::esvalue::{EsValueConvertible, EsValueFacade};
use crate::features;
use crate::quickjs_utils::{functions, objects};
use crate::quickjsruntime::{QuickJsRuntime, QJS_RT};
use hirofa_utils::single_threaded_event_queue::SingleThreadedEventQueue;
use log::error;
use std::sync::Arc;

use hirofa_utils::task_manager::TaskManager;

lazy_static! {
    /// a static Multithreaded taskmanager used to run rust ops async and multithreaded ( in at least 2 threads)
    static ref HELPER_TASKS: Arc<TaskManager> = Arc::new(TaskManager::new(std::cmp::max(2, num_cpus::get())));
}

pub struct EsRuntimeInner {
    event_queue: Arc<SingleThreadedEventQueue>,
}

pub struct EsRuntime {
    pub(crate) inner: Arc<EsRuntimeInner>,
}

impl EsRuntimeInner {
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

impl EsRuntime {
    pub(crate) fn new(builder: EsRuntimeBuilder) -> Arc<Self> {
        let ret = Arc::new(Self {
            inner: Arc::new(EsRuntimeInner {
                event_queue: SingleThreadedEventQueue::new(),
            }),
        });

        // run single job in eventQueue to init thread_local weak<rtref>

        let res = ret.add_to_event_queue_sync(|q_js_rt| features::init(q_js_rt));
        if res.is_err() {
            panic!("could not init features: {}", res.err().unwrap());
        }

        ret.inner.event_queue.exe_task(|| {
            QJS_RT.with(move |qjs_rt_rc| {
                let q_js_rt = &mut *qjs_rt_rc.borrow_mut();
                if builder.loader.is_some() {
                    q_js_rt.module_script_loader = Some(builder.loader.unwrap());
                }
            })
        });

        ret
    }

    pub fn builder() -> EsRuntimeBuilder {
        EsRuntimeBuilder::new()
    }

    /// Evaluate a script asynchronously
    pub fn eval(&self, script: EsScript) {
        self.add_to_event_queue(|qjs_rt| {
            let res = qjs_rt.eval(script);
            match res {
                Ok(_) => {}
                Err(e) => log::error!("error in async eval {}", e),
            }
        });
    }

    /// Evaluate a script and return the result synchronously
    /// # example
    /// ```rust
    /// use quickjs_es_runtime::esruntimebuilder::EsRuntimeBuilder;
    /// use quickjs_es_runtime::esscript::EsScript;
    /// let rt = EsRuntimeBuilder::new().build();
    /// let script = EsScript::new("my_file.es", "(9 * 3);");
    /// let res = rt.eval_sync(script).ok().expect("script failed");
    /// assert_eq!(res.get_i32(), 27);
    /// ```
    pub fn eval_sync(&self, script: EsScript) -> Result<EsValueFacade, EsError> {
        let inner_arc = self.inner.clone();
        self.add_to_event_queue_sync(move |qjs_rt| {
            let res = qjs_rt.eval(script);
            match res {
                Ok(val_ref) => EsValueFacade::from_jsval(qjs_rt, &val_ref, &inner_arc),
                Err(e) => Err(e),
            }
        })
    }

    /// run the garbage collector asynchronously
    pub fn gc(&self) {
        self.add_to_event_queue(|q_js_rt| q_js_rt.gc())
    }

    /// run the garbage collector and wait for it to be done
    pub fn gc_sync(&self) {
        self.add_to_event_queue_sync(|q_js_rt| q_js_rt.gc())
    }

    /// call a function in the engine and await the result
    /// # example
    /// ```rust
    /// use quickjs_es_runtime::esruntimebuilder::EsRuntimeBuilder;
    /// use quickjs_es_runtime::esscript::EsScript;
    /// use quickjs_es_runtime::esvalue::EsValueConvertible;
    /// let rt = EsRuntimeBuilder::new().build();
    /// let script = EsScript::new("my_file.es", "this.com = {my: {methodA: function(a, b){return a*b;}}};");
    /// rt.eval_sync(script).ok().expect("script failed");
    /// let res = rt.call_function_sync(vec!["com", "my"], "methodA", vec![7.to_es_value_facade(), 5.to_es_value_facade()]).ok().expect("func failed");
    /// assert_eq!(res.get_i32(), 35);
    /// ```
    pub fn call_function_sync(
        &self,
        namespace: Vec<&'static str>,
        func_name: &str,
        arguments: Vec<EsValueFacade>,
    ) -> Result<EsValueFacade, EsError> {
        let func_name_string = func_name.to_string();
        let inner_arc = self.inner.clone();
        self.add_to_event_queue_sync(move |q_js_rt| {
            let q_args = arguments
                .iter()
                .map(|arg| {
                    arg.to_js_value(q_js_rt)
                        .ok()
                        .expect("arg conversion failed")
                })
                .collect::<Vec<_>>();

            let res = q_js_rt.call_function(namespace, func_name_string.as_str(), q_args);
            match res {
                Ok(val_ref) => EsValueFacade::from_jsval(q_js_rt, &val_ref, &inner_arc),
                Err(e) => Err(e),
            }
        })
    }

    /// evaluate a module, you need if you want to compile a script that contains static imports
    /// e.g.
    /// ```javascript
    /// import {util} from 'file.mes';
    /// console.log(util(1, 2, 3));
    /// ```
    /// please note that the module is cached under the absolute path you passed in the EsScript object
    /// and thus you should take care to make the path unique (hence the absolute_ name)
    /// also to use this you need to build the EsRuntime with a module loader closure
    /// # example
    /// ```rust
    /// use quickjs_es_runtime::esruntimebuilder::EsRuntimeBuilder;
    /// use quickjs_es_runtime::esscript::EsScript;
    /// use quickjs_es_runtime::esvalue::EsValueConvertible;
    /// let rt = EsRuntimeBuilder::new().module_script_loader(|relative_file_path, name| {
    ///     // here you should analyze the relative_file_path, this is the absolute path of the script which contains the import statement
    ///     // if this is e.g. '/opt/files/my_module.mes' and the name is 'other_module.mes' then you should return
    ///     // EsScript object with '/opt/files/other_module.mes' as absolute path
    ///     let name = format!("/opt/files/{}", name);
    ///     // this is of course a bad impl, name might for example be '../files/other_module.mes'
    ///     Some(EsScript::new(name.as_str(), "export const util = function(a, b, c){return a+b+c;};"))
    /// }).build();
    /// let script = EsScript::new("/opt/files/my_module.mes", "import {util} from 'other_module.mes';
    /// console.log(util(1, 2, 3));");
    /// rt.eval_module(script);
    /// ```
    pub fn eval_module(&self, script: EsScript) {
        self.add_to_event_queue(|qjs_rt| {
            let res = qjs_rt.eval_module(script);
            match res {
                Ok(_) => {}
                Err(e) => log::error!("error in async eval {}", e),
            }
        });
    }

    /// evaluate a module and return result synchronously
    pub fn eval_module_sync(&self, script: EsScript) -> Result<EsValueFacade, EsError> {
        let inner_arc = self.inner.clone();
        self.add_to_event_queue_sync(move |qjs_rt| {
            let res = qjs_rt.eval_module(script);
            match res {
                Ok(val_ref) => EsValueFacade::from_jsval(qjs_rt, &val_ref, &inner_arc),
                Err(e) => Err(e),
            }
        })
    }

    /// this is how you add a closure to the worker thread which has an instance of the QuickJsRuntime
    /// this will run asynchronously
    /// # example
    /// ```rust
    /// use quickjs_es_runtime::esruntimebuilder::EsRuntimeBuilder;
    /// let rt = EsRuntimeBuilder::new().build();
    /// rt.add_to_event_queue(|q_js_rt| {
    ///     // here you are in the worker thread and you can use the quickjs_utils
    ///     q_js_rt.gc();
    /// });
    /// ```
    pub fn add_to_event_queue<C>(&self, consumer: C)
    where
        C: FnOnce(&QuickJsRuntime) + Send + 'static,
    {
        self.inner.add_to_event_queue(consumer)
    }

    /// this is how you add a closure to the worker thread which has an instance of the QuickJsRuntime
    /// this will run and return synchronously
    /// # example
    /// ```rust
    /// use quickjs_es_runtime::esruntimebuilder::EsRuntimeBuilder;
    /// use quickjs_es_runtime::esscript::EsScript;
    /// use quickjs_es_runtime::quickjs_utils::primitives;
    /// let rt = EsRuntimeBuilder::new().build();
    /// let res = rt.add_to_event_queue_sync(|q_js_rt| {
    ///     // here you are in the worker thread and you can use the quickjs_utils
    ///     let val_ref = q_js_rt.eval(EsScript::new("test.es", "(11 * 6);")).ok().expect("script failed");
    ///     primitives::to_i32(&val_ref).ok().expect("could not get i32")
    /// });
    /// assert_eq!(res, 66);
    /// ```
    pub fn add_to_event_queue_sync<C, R>(&self, consumer: C) -> R
    where
        C: FnOnce(&QuickJsRuntime) -> R + Send + 'static,
        R: Send + 'static,
    {
        self.inner.add_to_event_queue_sync(consumer)
    }

    /// this adds a rust function to JavaScript
    /// # Example
    /// ```rust
    /// use quickjs_es_runtime::esruntimebuilder::EsRuntimeBuilder;
    /// use quickjs_es_runtime::esscript::EsScript;
    /// use quickjs_es_runtime::quickjs_utils::primitives;
    /// use quickjs_es_runtime::esvalue::{EsValueFacade, EsValueConvertible};
    /// let rt = EsRuntimeBuilder::new().build();
    /// rt.set_function(vec!["com", "mycompany", "util"], "methodA", |args: Vec<EsValueFacade>|{
    ///     let a = args[0].get_i32();
    ///     let b = args[1].get_i32();
    ///     Ok(a * b)
    /// });
    /// let res = rt.eval_sync(EsScript::new("test.es", "let a = com.mycompany.util.methodA(13, 17); a * 2;")).ok().expect("script failed");
    /// assert_eq!(res.get_i32(), (13*17*2));
    /// ```
    pub fn set_function<F, E>(
        &self,
        namespace: Vec<&'static str>,
        name: &str,
        function: F,
    ) -> Result<(), EsError>
    where
        F: Fn(Vec<EsValueFacade>) -> Result<E, EsError> + Send + 'static,
        E: EsValueConvertible + Send + 'static,
    {
        let rti_ref = self.inner.clone();
        let name = name.to_string();
        self.add_to_event_queue_sync(move |q_js_rt| {
            let ns = objects::get_namespace(q_js_rt, namespace, true)?;
            let func = functions::new_function(
                q_js_rt,
                name.as_str(),
                move |_this_ref, args| {
                    QuickJsRuntime::do_with(|q_js_rt| {
                        let mut args_facades = vec![];

                        for arg_ref in args {
                            args_facades.push(EsValueFacade::from_jsval(
                                q_js_rt,
                                &arg_ref,
                                &rti_ref.clone(),
                            )?);
                        }

                        let res = function(args_facades);

                        match res {
                            Ok(val_ref) => val_ref.to_es_value_facade().to_js_value(q_js_rt),
                            Err(e) => Err(e),
                        }
                    })
                },
                1,
            )?;

            objects::set_property2(q_js_rt, &ns, name.as_str(), &func, 0)?;

            Ok(())
        })
    }

    /// add a task the the "helper" thread pool
    pub fn add_helper_task<T>(task: T)
    where
        T: FnOnce() + Send + 'static,
    {
        log::trace!("adding a helper task");
        HELPER_TASKS.add_task(task);
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
        EsRuntime::builder()
            .module_script_loader(|_rel, name| {
                if name.eq("invalid.mes") {
                    None
                } else {
                    Some(EsScript::new(name, "export const foo = 'bar';\nexport const mltpl = function(a, b){return a*b;}; globalThis;"))
                }
            })
            .build()
    }

    #[test]
    fn test_func() {
        let rt: Arc<EsRuntime> = TEST_ESRT.clone();
        let res = rt.set_function(vec!["nl", "my", "utils"], "methodA", |args| {
            if args.len() != 2 || !args.get(0).unwrap().is_i32() || !args.get(1).unwrap().is_i32() {
                Err(EsError::new_str(
                    "i'd realy like 2 args of the int32 kind please",
                ))
            } else {
                let a = args.get(0).unwrap().get_i32();
                let b = args.get(1).unwrap().get_i32();
                Ok(a * b)
            }
        });

        match res {
            Ok(_) => {}
            Err(e) => {
                panic!("set_function failed: {}", e);
            }
        }

        let res = rt.eval_sync(EsScript::new(
            "test_func.es",
            "(nl.my.utils.methodA(13, 56));",
        ));

        match res {
            Ok(val) => {
                assert!(val.is_i32());
                assert_eq!(val.get_i32(), 13 * 56);
            }
            Err(e) => {
                panic!("test_func.es failed: {}", e);
            }
        }
    }

    #[test]
    fn test_eval_sync() {
        let rt: Arc<EsRuntime> = TEST_ESRT.clone();
        let res = rt.eval_sync(EsScript::new("test.es", "console.log('foo bar');"));

        match res {
            Ok(_) => {}
            Err(e) => {
                panic!("eval failed: {}", e);
            }
        }

        let res = rt
            .eval_sync(EsScript::new("test.es", "(2 * 7);"))
            .ok()
            .expect("script failed");

        assert_eq!(res.get_i32(), 14);
    }

    #[test]
    fn test_promise() {
        let rt: Arc<EsRuntime> = TEST_ESRT.clone();

        let res = rt.eval_sync(EsScript::new(
            "testp2.es",
            "let test_promise_P = (new Promise(function(res, rej) {console.log('before res');res(123);console.log('after res');}).then(function (a) {console.log('prom ressed to ' + a);}).catch(function(x) {console.log('p.ca ex=' + x);}))",
        ));

        match res {
            Ok(_) => {}
            Err(e) => panic!("p script failed: {}", e),
        }
        std::thread::sleep(Duration::from_secs(1));
    }

    #[test]
    fn test_module_sync() {
        log::info!("> test_module_sync");

        let rt = &TEST_ESRT;
        debug!("test static import");
        let res: Result<EsValueFacade, EsError> = rt.eval_module_sync(EsScript::new(
            "test.es",
            "import {foo} from 'test_module.mes';\n console.log('static imp foo = ' + foo);",
        ));

        match res {
            Ok(_) => {
                log::debug!("static import ok");
            }
            Err(e) => {
                log::error!("static import failed: {}", e);
            }
        }

        debug!("test dynamic import");
        let res: Result<EsValueFacade, EsError> = rt.eval_sync(EsScript::new(
            "test_dyn.es",
            "console.log('about to load dynamic module');let dyn_p = import('test_module.mes');dyn_p.then(function (some) {console.log('after dyn');console.log('after dyn ' + typeof some);console.log('mltpl 5, 7 = ' + some.mltpl(5, 7));});dyn_p.catch(function (x) {console.log('imp.cat x=' + x);});console.log('dyn done');",
        ));

        match res {
            Ok(_) => {
                log::debug!("dynamic import ok");
            }
            Err(e) => {
                log::error!("dynamic import failed: {}", e);
            }
        }
        std::thread::sleep(Duration::from_secs(1));

        log::info!("< test_module_sync");
    }
}
