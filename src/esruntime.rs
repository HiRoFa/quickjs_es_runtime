use crate::eserror::EsError;
use crate::esruntimebuilder::EsRuntimeBuilder;
use crate::esscript::EsScript;
use crate::esvalue::EsValueFacade;
use crate::features;
use crate::features::fetch::request::FetchRequest;
use crate::features::fetch::response::FetchResponse;
use crate::quickjs_utils::{functions, objects};
use crate::quickjscontext::QuickJsContext;
use crate::quickjsruntime::{NativeModuleLoaderAdapter, QuickJsRuntime, ScriptModuleLoaderAdapter};
use hirofa_utils::eventloop::EventLoop;
use hirofa_utils::task_manager::TaskManager;
use libquickjs_sys as q;
use std::future::Future;
use std::rc::Rc;
use std::sync::{Arc, Weak};
use tokio::task::JoinError;

lazy_static! {
    /// a static Multithreaded task manager used to run rust ops async and multithreaded ( in at least 2 threads)
    static ref HELPER_TASKS: TaskManager = TaskManager::new(std::cmp::max(2, num_cpus::get()));
}

pub type FetchResponseProvider =
    dyn Fn(&FetchRequest) -> Box<dyn FetchResponse + Send> + Send + Sync + 'static;

pub trait ScriptPreProcessor {
    fn process(&self, script: EsScript) -> Result<EsScript, EsError> {
        Ok(script)
    }
}

impl Drop for EsRuntime {
    fn drop(&mut self) {
        log::trace!("> EsRuntime::drop");
        self.clear_contexts();
        log::trace!("< EsRuntime::drop");
    }
}

/// EsRuntime is the main public struct representing a JavaScript runtime.
/// You can construct a new EsRuntime by using the [EsRuntimeBuilder] struct
/// # Example
/// ```rust
/// use quickjs_runtime::esruntimebuilder::EsRuntimeBuilder;
/// let rt = EsRuntimeBuilder::new().build();
/// ```

pub struct EsRuntime {
    event_loop: EventLoop,
    fetch_response_provider: Option<Box<FetchResponseProvider>>,
}

impl EsRuntime {
    pub(crate) fn new(mut builder: EsRuntimeBuilder) -> Arc<Self> {
        let fetch_response_provider =
            std::mem::replace(&mut builder.opt_fetch_response_provider, None);

        let ret = Arc::new(Self {
            event_loop: EventLoop::new(),
            fetch_response_provider,
        });

        ret.exe_task(|| {
            let rt_ptr = unsafe { q::JS_NewRuntime() };
            let rt = QuickJsRuntime::new(rt_ptr);
            QuickJsRuntime::init_rt_for_current_thread(rt);
        });

        // init ref in q_js_rt
        let rt_ref = ret.clone();
        ret.exe_task(move || {
            QuickJsRuntime::do_with_mut(move |m_q_js_rt| {
                m_q_js_rt.init_rt_ref(rt_ref);
            })
        });

        // run single job in eventQueue to init thread_local weak<rtref>

        let res = features::init(&ret);
        if res.is_err() {
            panic!("could not init features: {}", res.err().unwrap());
        }

        if let Some(interval) = builder.opt_gc_interval {
            let e_ref: Weak<EsRuntime> = Arc::downgrade(&ret);
            std::thread::spawn(move || loop {
                std::thread::sleep(interval);
                if let Some(rt) = e_ref.upgrade() {
                    log::trace!("running gc from gc interval thread");
                    rt.gc_sync();
                } else {
                    break;
                }
            });
        }

        let init_hooks: Vec<_> = builder.runtime_init_hooks.drain(..).collect();

        ret.exe_task(|| {
            QuickJsRuntime::do_with_mut(|q_js_rt| {
                for native_module_loader in builder.native_module_loaders {
                    q_js_rt.add_native_module_loader(NativeModuleLoaderAdapter::new(
                        native_module_loader,
                    ));
                }
                for script_module_loader in builder.script_module_loaders {
                    q_js_rt.add_script_module_loader(ScriptModuleLoaderAdapter::new(
                        script_module_loader,
                    ));
                }
                q_js_rt.script_pre_processors = builder.script_pre_processors;

                if let Some(limit) = builder.opt_memory_limit_bytes {
                    unsafe {
                        q::JS_SetMemoryLimit(q_js_rt.runtime, limit as _);
                    }
                }
                if let Some(threshold) = builder.opt_gc_threshold {
                    unsafe {
                        q::JS_SetGCThreshold(q_js_rt.runtime, threshold as _);
                    }
                }
                if let Some(stack_size) = builder.opt_max_stack_size {
                    unsafe {
                        q::JS_SetMaxStackSize(q_js_rt.runtime, stack_size as _);
                    }
                }
                if let Some(interrupt_handler) = builder.interrupt_handler {
                    q_js_rt.set_interrupt_handler(interrupt_handler);
                }
            })
        });

        for hook in init_hooks {
            match hook(&ret) {
                Ok(_) => {}
                Err(e) => {
                    panic!("runtime_init_hook failed: {}", e);
                }
            }
        }

        ret
    }

    pub(crate) fn clear_contexts(&self) {
        log::trace!("EsRuntime::clear_contexts");
        self.exe_task_in_event_loop(|| {
            let context_ids = QuickJsRuntime::get_context_ids();
            for id in context_ids {
                QuickJsRuntime::remove_context(id.as_str());
            }
        });
    }

    /// this can be used to run a function in the event_queue thread for the QuickJSRuntime
    /// without borrowing the q_js_rt
    pub fn add_task_to_event_loop_void<C>(&self, task: C)
    where
        C: FnOnce() + Send + 'static,
    {
        self.event_loop.add_void(move || {
            task();
            EventLoop::add_local_void(|| {
                QuickJsRuntime::do_with(|q_js_rt| {
                    q_js_rt.run_pending_jobs_if_any();
                })
            })
        });
    }

    pub fn exe_task_in_event_loop<C, R: Send + 'static>(&self, task: C) -> R
    where
        C: FnOnce() -> R + Send + 'static,
    {
        self.event_loop.exe(move || {
            let res = task();
            EventLoop::add_local_void(|| {
                QuickJsRuntime::do_with(|q_js_rt| {
                    q_js_rt.run_pending_jobs_if_any();
                })
            });
            res
        })
    }

    pub fn add_task_to_event_loop<C, R: Send + 'static>(&self, task: C) -> impl Future<Output = R>
    where
        C: FnOnce() -> R + Send + 'static,
    {
        self.event_loop.add(move || {
            let res = task();
            EventLoop::add_local_void(|| {
                QuickJsRuntime::do_with(|q_js_rt| {
                    q_js_rt.run_pending_jobs_if_any();
                });
            });
            res
        })
    }

    /// this is how you add a closure to the worker thread which has an instance of the QuickJsRuntime
    /// this will run asynchronously
    /// # example
    /// ```rust
    /// use quickjs_runtime::esruntimebuilder::EsRuntimeBuilder;
    /// let rt = EsRuntimeBuilder::new().build();
    /// rt.add_rt_task_to_event_loop(|q_js_rt| {
    ///     // here you are in the worker thread and you can use the quickjs_utils
    ///     q_js_rt.gc();
    /// });
    /// ```
    pub fn add_rt_task_to_event_loop<C, R: Send + 'static>(
        &self,
        consumer: C,
    ) -> impl Future<Output = R>
    where
        C: FnOnce(&QuickJsRuntime) -> R + Send + 'static,
    {
        self.add_task_to_event_loop(|| QuickJsRuntime::do_with(consumer))
    }

    pub fn add_rt_task_to_event_loop_void<C>(&self, consumer: C)
    where
        C: FnOnce(&QuickJsRuntime) + Send + 'static,
    {
        self.add_task_to_event_loop_void(|| QuickJsRuntime::do_with(consumer))
    }

    /// used to add tasks from the worker threads which require run_pending_jobs_if_any to run after it
    pub(crate) fn add_local_task_to_event_loop<C>(consumer: C)
    where
        C: FnOnce(&QuickJsRuntime) + 'static,
    {
        EventLoop::add_local_void(move || {
            QuickJsRuntime::do_with(|q_js_rt| {
                consumer(q_js_rt);
            });
            EventLoop::add_local_void(|| {
                QuickJsRuntime::do_with(|q_js_rt| {
                    q_js_rt.run_pending_jobs_if_any();
                })
            })
        });
    }

    #[allow(clippy::borrowed_box)]
    pub fn get_fetch_response_provider(&self) -> Option<&Box<FetchResponseProvider>> {
        self.fetch_response_provider.as_ref()
    }

    pub fn builder() -> EsRuntimeBuilder {
        EsRuntimeBuilder::new()
    }

    /// this can be used to run a function in the event_queue thread for the QuickJSRuntime
    /// without borrowing the q_js_rt
    pub fn exe_task<C, R: Send + 'static>(&self, task: C) -> R
    where
        C: FnOnce() -> R + Send + 'static,
    {
        self.exe_task_in_event_loop(task)
    }

    /// Evaluate a script asynchronously
    pub async fn eval(&self, script: EsScript) -> Result<EsValueFacade, EsError> {
        self.add_rt_task_to_event_loop(|q_js_rt| {
            let q_ctx = q_js_rt.get_main_context();
            let res = q_ctx.eval(script);
            match res {
                Ok(js) => EsValueFacade::from_jsval(q_ctx, &js),
                Err(e) => Err(e),
            }
        })
        .await
    }

    /// Evaluate a script and return the result synchronously
    /// # example
    /// ```rust
    /// use quickjs_runtime::esruntimebuilder::EsRuntimeBuilder;
    /// use quickjs_runtime::esscript::EsScript;
    /// let rt = EsRuntimeBuilder::new().build();
    /// let script = EsScript::new("my_file.es", "(9 * 3);");
    /// let res = rt.eval_sync(script).ok().expect("script failed");
    /// assert_eq!(res.get_i32(), 27);
    /// ```
    pub fn eval_sync(&self, script: EsScript) -> Result<EsValueFacade, EsError> {
        self.exe_rt_task_in_event_loop(move |q_js_rt| {
            let q_ctx = q_js_rt.get_main_context();
            let res = q_ctx.eval(script);
            match res {
                Ok(val_ref) => EsValueFacade::from_jsval(q_ctx, &val_ref),
                Err(e) => Err(e),
            }
        })
    }

    /// run the garbage collector asynchronously
    pub async fn gc(&self) {
        self.add_rt_task_to_event_loop(|q_js_rt| q_js_rt.gc()).await
    }

    /// run the garbage collector and wait for it to be done
    pub fn gc_sync(&self) {
        self.exe_rt_task_in_event_loop(|q_js_rt| q_js_rt.gc())
    }

    /// call a function in the engine and await the result
    /// # example
    /// ```rust
    /// use quickjs_runtime::esruntimebuilder::EsRuntimeBuilder;
    /// use quickjs_runtime::esscript::EsScript;
    /// use quickjs_runtime::es_args;
    /// use quickjs_runtime::esvalue::EsValueConvertible;
    /// use quickjs_runtime::esvalue::EsValueFacade;
    /// let rt = EsRuntimeBuilder::new().build();
    /// let script = EsScript::new("my_file.es", "this.com = {my: {methodA: function(a, b, someStr, someBool){return a*b;}}};");
    /// rt.eval_sync(script).ok().expect("script failed");
    /// let res = rt.call_function_sync(vec!["com", "my"], "methodA", vec![7i32.to_es_value_facade(), 5i32.to_es_value_facade(), "abc".to_string().to_es_value_facade(), true.to_es_value_facade()]).ok().expect("func failed");
    /// assert_eq!(res.get_i32(), 35);
    /// ```
    pub fn call_function_sync(
        &self,
        namespace: Vec<&'static str>,
        func_name: &str,
        mut arguments: Vec<EsValueFacade>,
    ) -> Result<EsValueFacade, EsError> {
        let func_name_string = func_name.to_string();

        self.exe_rt_task_in_event_loop(move |q_js_rt| {
            let q_ctx = q_js_rt.get_main_context();

            let mut q_args = vec![];
            for arg in &mut arguments {
                q_args.push(arg.as_js_value(q_ctx)?);
            }

            let res = q_ctx.call_function(namespace, func_name_string.as_str(), q_args);
            match res {
                Ok(val_ref) => EsValueFacade::from_jsval(q_ctx, &val_ref),
                Err(e) => Err(e),
            }
        })
    }

    /// call a function in the engine asynchronously
    /// N.B. func_name is not a &str because of https://github.com/rust-lang/rust/issues/56238 (i think)
    /// # example
    /// ```rust
    /// use quickjs_runtime::esruntimebuilder::EsRuntimeBuilder;
    /// use quickjs_runtime::esscript::EsScript;
    /// use quickjs_runtime::esvalue::EsValueConvertible;
    /// let rt = EsRuntimeBuilder::new().build();
    /// let script = EsScript::new("my_file.es", "this.com = {my: {methodA: function(a, b){return a*b;}}};");
    /// rt.eval_sync(script).ok().expect("script failed");
    /// rt.call_function(vec!["com", "my"], "methodA".to_string(), vec![7.to_es_value_facade(), 5.to_es_value_facade()]);
    /// ```
    pub async fn call_function(
        &self,
        namespace: Vec<&'static str>,
        func_name: String,
        mut arguments: Vec<EsValueFacade>,
    ) -> Result<EsValueFacade, EsError> {
        let func_name_string = func_name.to_string();

        self.add_rt_task_to_event_loop(move |q_js_rt| {
            let q_ctx = q_js_rt.get_main_context();
            let mut q_args = vec![];
            for arg in &mut arguments {
                match arg.as_js_value(q_ctx) {
                    Ok(js_arg) => q_args.push(js_arg),
                    Err(err) => log::error!(
                        "error occurred in async esruntime::call_function closure: {}",
                        err
                    ),
                }
            }

            let res = q_ctx.call_function(namespace, func_name_string.as_str(), q_args);
            match res {
                Ok(js_ref) => EsValueFacade::from_jsval(q_ctx, &js_ref),
                Err(e) => Err(e),
            }
        })
        .await
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
    /// use quickjs_runtime::esruntimebuilder::EsRuntimeBuilder;
    /// use quickjs_runtime::esscript::EsScript;
    /// use quickjs_runtime::esvalue::EsValueConvertible;
    /// use quickjs_runtime::quickjsruntime::ScriptModuleLoader;
    /// struct TestModuleLoader {}
    /// impl ScriptModuleLoader for TestModuleLoader {
    ///     fn normalize_path(&self,ref_path: &str,path: &str) -> Option<String> {
    ///         Some(path.to_string())
    ///     }
    ///
    ///     fn load_module(&self,absolute_path: &str) -> String {
    ///         "export const util = function(a, b, c){return a+b+c;};".to_string()
    ///     }
    /// }
    /// let rt = EsRuntimeBuilder::new().script_module_loader(Box::new(TestModuleLoader{})).build();
    /// let script = EsScript::new("/opt/files/my_module.mes", "import {util} from 'other_module.mes';\n
    /// console.log(util(1, 2, 3));");
    /// rt.eval_module(script);
    /// ```
    pub async fn eval_module(&self, script: EsScript) {
        self.add_rt_task_to_event_loop(|q_js_rt| {
            let q_ctx = q_js_rt.get_main_context();
            let res = q_ctx.eval_module(script);
            match res {
                Ok(_) => {}
                Err(e) => log::error!("error in async eval {}", e),
            }
        })
        .await
    }

    /// evaluate a module and return result synchronously
    pub fn eval_module_sync(&self, script: EsScript) -> Result<EsValueFacade, EsError> {
        self.exe_rt_task_in_event_loop(move |q_js_rt| {
            let q_ctx = q_js_rt.get_main_context();
            let res = q_ctx.eval_module(script);
            match res {
                Ok(val_ref) => EsValueFacade::from_jsval(q_ctx, &val_ref),
                Err(e) => Err(e),
            }
        })
    }

    /// this is how you add a closure to the worker thread which has an instance of the QuickJsRuntime
    /// this will run and return synchronously
    /// # example
    /// ```rust
    /// use quickjs_runtime::esruntimebuilder::EsRuntimeBuilder;
    /// use quickjs_runtime::esscript::EsScript;
    /// use quickjs_runtime::quickjs_utils::primitives;
    /// let rt = EsRuntimeBuilder::new().build();
    /// let res = rt.exe_rt_task_in_event_loop(|q_js_rt| {
    ///     let q_ctx = q_js_rt.get_main_context();
    ///     // here you are in the worker thread and you can use the quickjs_utils
    ///     let val_ref = q_ctx.eval(EsScript::new("test.es", "(11 * 6);")).ok().expect("script failed");
    ///     primitives::to_i32(&val_ref).ok().expect("could not get i32")
    /// });
    /// assert_eq!(res, 66);
    /// ```
    pub fn exe_rt_task_in_event_loop<C, R>(&self, consumer: C) -> R
    where
        C: FnOnce(&QuickJsRuntime) -> R + Send + 'static,
        R: Send + 'static,
    {
        self.exe_task_in_event_loop(|| QuickJsRuntime::do_with(consumer))
    }

    /// this adds a rust function to JavaScript, it is added for all current and future contexts
    /// # Example
    /// ```rust
    /// use quickjs_runtime::esruntimebuilder::EsRuntimeBuilder;
    /// use quickjs_runtime::esscript::EsScript;
    /// use quickjs_runtime::quickjs_utils::primitives;
    /// use quickjs_runtime::esvalue::{EsValueFacade, EsValueConvertible};
    /// let rt = EsRuntimeBuilder::new().build();
    /// rt.set_function(vec!["com", "mycompany", "util"], "methodA", |q_ctx, args: Vec<EsValueFacade>|{
    ///     let a = args[0].get_i32();
    ///     let b = args[1].get_i32();
    ///     Ok((a * b).to_es_value_facade())
    /// });
    /// let res = rt.eval_sync(EsScript::new("test.es", "let a = com.mycompany.util.methodA(13, 17); a * 2;")).ok().expect("script failed");
    /// assert_eq!(res.get_i32(), (13*17*2));
    /// ```
    pub fn set_function<F>(
        &self,
        namespace: Vec<&'static str>,
        name: &str,
        function: F,
    ) -> Result<(), EsError>
    where
        F: Fn(&QuickJsContext, Vec<EsValueFacade>) -> Result<EsValueFacade, EsError>
            + Send
            + 'static,
    {
        let name = name.to_string();
        self.exe_rt_task_in_event_loop(move |q_js_rt| {
            let func_rc = Rc::new(function);
            let name = name.to_string();

            q_js_rt.add_context_init_hook(move |_q_js_rt, q_ctx| {
                let ns = objects::get_namespace_q(q_ctx, namespace.clone(), true)?;

                let func_rc = func_rc.clone();

                let func = functions::new_function_q(
                    q_ctx,
                    name.as_str(),
                    move |q_ctx, _this_ref, args| {
                        let mut args_facades = vec![];

                        for arg_ref in args {
                            args_facades.push(EsValueFacade::from_jsval(q_ctx, &arg_ref)?);
                        }

                        let res = func_rc(q_ctx, args_facades);

                        match res {
                            Ok(mut val_esvf) => val_esvf.as_js_value(q_ctx),
                            Err(e) => Err(e),
                        }
                    },
                    1,
                )?;

                objects::set_property2_q(q_ctx, &ns, name.as_str(), &func, 0)?;

                Ok(())
            })
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

    /// add an async task the the "helper" thread pool
    pub fn add_helper_task_async<R: Send + 'static, T: Future<Output = R> + Send + 'static>(
        task: T,
    ) -> impl Future<Output = Result<R, JoinError>> {
        log::trace!("adding an async helper task");
        HELPER_TASKS.add_task_async(task)
    }

    /// create a new context besides the always existing main_context
    /// # todo
    /// EsRuntime needs some more pub methods using context like eval / call_func
    /// # Example
    /// ```
    /// use quickjs_runtime::esruntimebuilder::EsRuntimeBuilder;
    /// use quickjs_runtime::esscript::EsScript;
    /// let rt = EsRuntimeBuilder::new().build();
    /// rt.create_context("my_context");
    /// rt.exe_rt_task_in_event_loop(|q_js_rt| {
    ///    let my_ctx = q_js_rt.get_context("my_context");
    ///    my_ctx.eval(EsScript::new("ctx_test.es", "this.myVar = 'only exists in my_context';"));
    /// });
    /// ```
    pub fn create_context(&self, id: &str) -> Result<(), EsError> {
        let id = id.to_string();
        self.event_loop
            .exe(move || QuickJsRuntime::create_context(id.as_str()))
    }

    /// drop a context which was created earlier with a call to [create_context()](struct.EsRuntime.html#method.create_context)
    pub fn drop_context(&self, id: &str) {
        let id = id.to_string();
        self.event_loop
            .exe(move || QuickJsRuntime::remove_context(id.as_str()))
    }
}

#[cfg(test)]
pub mod tests {
    use crate::eserror::EsError;
    use crate::esruntime::EsRuntime;
    use crate::esscript::EsScript;
    use crate::esvalue::{EsValueConvertible, EsValueFacade};
    use crate::quickjs_utils::{primitives, promises};
    use crate::quickjscontext::QuickJsContext;
    use crate::quickjsruntime::{NativeModuleLoader, ScriptModuleLoader};
    use crate::valueref::JSValueRef;
    use backtrace::Backtrace;
    use futures::executor::block_on;
    use log::debug;
    use log::LevelFilter;
    use std::panic;
    use std::sync::Arc;
    use std::time::Duration;

    struct TestNativeModuleLoader {}
    struct TestScriptModuleLoader {}

    impl NativeModuleLoader for TestNativeModuleLoader {
        fn has_module(&self, _q_ctx: &QuickJsContext, module_name: &str) -> bool {
            module_name.starts_with("greco://")
        }

        fn get_module_export_names(
            &self,
            _q_ctx: &QuickJsContext,
            _module_name: &str,
        ) -> Vec<&str> {
            vec!["a", "b", "c"]
        }

        fn get_module_exports(
            &self,
            _q_ctx: &QuickJsContext,
            _module_name: &str,
        ) -> Vec<(&str, JSValueRef)> {
            vec![
                ("a", primitives::from_i32(1234)),
                ("b", primitives::from_i32(64834)),
                ("c", primitives::from_i32(333)),
            ]
        }
    }

    impl ScriptModuleLoader for TestScriptModuleLoader {
        fn normalize_path(&self, _ref_path: &str, path: &str) -> Option<String> {
            if path.eq("notfound.mes") || path.starts_with("greco://") {
                None
            } else if path.eq("invalid.mes") {
                Some(path.to_string())
            } else {
                Some(path.to_string())
            }
        }

        fn load_module(&self, absolute_path: &str) -> String {
            if absolute_path.eq("notfound.mes") || absolute_path.starts_with("greco://") {
                panic!("tht realy should not happen");
            } else if absolute_path.eq("invalid.mes") {
                "I am the great cornholio! thou'gh shalt&s not p4arse mie!".to_string()
            } else {
                "export const foo = 'bar';\nexport const mltpl = function(a, b){return a*b;}; globalThis;".to_string()
            }
        }
    }

    #[test]
    fn test_rt_drop() {
        let rt = init_test_rt();
        log::trace!("before drop");

        drop(rt);
        log::trace!("after before drop");
        std::thread::sleep(Duration::from_secs(5));
        log::trace!("after sleep");
    }

    pub fn init_test_rt() -> Arc<EsRuntime> {
        panic::set_hook(Box::new(|panic_info| {
            let backtrace = Backtrace::new();
            println!(
                "thread panic occurred: {}\nbacktrace: {:?}",
                panic_info, backtrace
            );
            log::error!(
                "thread panic occurred: {}\nbacktrace: {:?}",
                panic_info,
                backtrace
            );
        }));

        simple_logging::log_to_file("esruntime.log", LevelFilter::max())
            .ok()
            .expect("could not init logger");

        EsRuntime::builder()
            .gc_interval(Duration::from_secs(1))
            .max_stack_size(u64::MAX)
            .script_module_loader(Box::new(TestScriptModuleLoader {}))
            .native_module_loader(Box::new(TestNativeModuleLoader {}))
            .build()
    }

    #[test]
    fn test_func() {
        let rt = init_test_rt();
        let res = rt.set_function(vec!["nl", "my", "utils"], "methodA", |_q_ctx, args| {
            if args.len() != 2 || !args.get(0).unwrap().is_i32() || !args.get(1).unwrap().is_i32() {
                Err(EsError::new_str(
                    "i'd realy like 2 args of the int32 kind please",
                ))
            } else {
                let a = args.get(0).unwrap().get_i32();
                let b = args.get(1).unwrap().get_i32();
                Ok((a * b).to_es_value_facade())
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
        let rt: Arc<EsRuntime> = init_test_rt();
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
    fn t1234() {
        // test stack overflow
        let rt: Arc<EsRuntime> = init_test_rt();

        rt.exe_rt_task_in_event_loop(|q_js_rt| {
            //q_js_rt.run_pending_jobs_if_any();
            let q_ctx = q_js_rt.get_main_context();
            let r = q_ctx.eval(EsScript::new(
                "test_async.es",
                "let f = async function(){let p = new Promise((resolve, reject) => {resolve(12345);}); const p2 = await p; return p2}; f();",
            )).ok().unwrap();
            log::trace!("tag = {}", r.get_tag());
            //std::thread::sleep(Duration::from_secs(1));

            assert!(promises::is_promise_q(q_ctx, &r));

            if promises::is_promise_q(q_ctx, &r) {
                log::info!("r IS a Promise");
            } else {
                log::error!("r is NOT a Promise");
            }

            std::thread::sleep(Duration::from_secs(1));

            //q_js_rt.run_pending_jobs_if_any();
        });
        rt.exe_rt_task_in_event_loop(|q_js_rt| {
            q_js_rt.run_pending_jobs_if_any();
        });

        std::thread::sleep(Duration::from_secs(1));
    }

    #[test]
    fn test_eval_await() {
        let rt: Arc<EsRuntime> = init_test_rt();

        let res = rt.eval_sync(EsScript::new(
            "test_async.es",
            "{let f = async function(){let p = new Promise((resolve, reject) => {resolve(12345);}); const p2 = await p; return p2}; f()};",
        ));

        match res {
            Ok(esvf) => {
                assert!(esvf.is_promise());
                let p_res = esvf.get_promise_result_sync();
                if p_res.is_err() {
                    panic!("{:?}", p_res.err().unwrap());
                }
                let res = p_res.ok().unwrap();
                assert!(res.is_i32());
                assert_eq!(res.get_i32(), 12345);
            }
            Err(e) => {
                panic!("eval failed: {}", e);
            }
        }
    }

    #[test]
    fn test_promise() {
        let rt: Arc<EsRuntime> = init_test_rt();

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

        let rt = init_test_rt();
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

    async fn test_async1() -> i32 {
        let rt = init_test_rt();
        let a = rt.eval(EsScript::new("test_async.es", "122 + 1;")).await;
        a.ok().expect("script failed").get_i32()
    }

    #[test]
    fn test_async() {
        let fut = test_async1();
        let res = block_on(fut);
        assert_eq!(res, 123);
    }

    #[test]
    fn test_macro() {
        let _args = es_args!(1, 2i32, true, "sdf".to_string());
    }
}
