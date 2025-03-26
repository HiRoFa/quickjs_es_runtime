//! contains the QuickJsRuntimeFacade

use crate::builder::QuickJsRuntimeBuilder;
use crate::jsutils::{JsError, Script};
use crate::quickjs_utils::{functions, objects};
use crate::quickjsrealmadapter::QuickJsRealmAdapter;
use crate::quickjsruntimeadapter::{
    CompiledModuleLoaderAdapter, MemoryUsage, NativeModuleLoaderAdapter, QuickJsRuntimeAdapter,
    ScriptModuleLoaderAdapter, QJS_RT,
};
use crate::quickjsvalueadapter::QuickJsValueAdapter;
use crate::reflection;
use crate::values::JsValueFacade;
use hirofa_utils::eventloop::EventLoop;
use hirofa_utils::task_manager::TaskManager;
use libquickjs_sys as q;
use std::future::Future;
use std::pin::Pin;
use std::rc::Rc;
use std::sync::{Arc, Weak};
use tokio::task::JoinError;

lazy_static! {
    /// a static Multithreaded task manager used to run rust ops async and multithreaded ( in at least 2 threads)
    static ref HELPER_TASKS: TaskManager = TaskManager::new(std::cmp::max(2, num_cpus::get()));
}

impl Drop for QuickJsRuntimeFacade {
    fn drop(&mut self) {
        log::trace!("> EsRuntime::drop");
        self.clear_contexts();
        log::trace!("< EsRuntime::drop");
    }
}

pub struct QuickjsRuntimeFacadeInner {
    event_loop: EventLoop,
}

impl QuickjsRuntimeFacadeInner {
    /// this is how you add a closure to the worker thread which has an instance of the QuickJsRuntime
    /// this will run and return synchronously
    /// # example
    /// ```rust
    /// use quickjs_runtime::builder::QuickJsRuntimeBuilder;
    /// use quickjs_runtime::jsutils::Script;
    /// use quickjs_runtime::quickjs_utils::primitives;
    /// let rt = QuickJsRuntimeBuilder::new().build();
    /// let res = rt.exe_rt_task_in_event_loop(|q_js_rt| {
    ///     let q_ctx = q_js_rt.get_main_realm();
    ///     // here you are in the worker thread and you can use the quickjs_utils
    ///     let val_ref = q_ctx.eval(Script::new("test.es", "(11 * 6);")).ok().expect("script failed");
    ///     primitives::to_i32(&val_ref).ok().expect("could not get i32")
    /// });
    /// assert_eq!(res, 66);
    /// ```
    pub fn exe_rt_task_in_event_loop<C, R>(&self, consumer: C) -> R
    where
        C: FnOnce(&QuickJsRuntimeAdapter) -> R + Send + 'static,
        R: Send + 'static,
    {
        self.exe_task_in_event_loop(|| QuickJsRuntimeAdapter::do_with(consumer))
    }

    /// this is how you add a closure to the worker thread which has an instance of the QuickJsRuntime
    /// this will run asynchronously
    /// # example
    /// ```rust
    /// use quickjs_runtime::builder::QuickJsRuntimeBuilder;
    /// let rt = QuickJsRuntimeBuilder::new().build();
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
        C: FnOnce(&QuickJsRuntimeAdapter) -> R + Send + 'static,
    {
        self.add_task_to_event_loop(|| QuickJsRuntimeAdapter::do_with(consumer))
    }

    pub fn add_rt_task_to_event_loop_void<C>(&self, consumer: C)
    where
        C: FnOnce(&QuickJsRuntimeAdapter) + Send + 'static,
    {
        self.add_task_to_event_loop_void(|| QuickJsRuntimeAdapter::do_with(consumer))
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
                QuickJsRuntimeAdapter::do_with(|q_js_rt| {
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
                QuickJsRuntimeAdapter::do_with(|q_js_rt| {
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
                QuickJsRuntimeAdapter::do_with(|q_js_rt| {
                    q_js_rt.run_pending_jobs_if_any();
                });
            });
            res
        })
    }

    /// used to add tasks from the worker threads which require run_pending_jobs_if_any to run after it
    #[allow(dead_code)]
    pub(crate) fn add_local_task_to_event_loop<C>(consumer: C)
    where
        C: FnOnce(&QuickJsRuntimeAdapter) + 'static,
    {
        EventLoop::add_local_void(move || {
            QuickJsRuntimeAdapter::do_with(|q_js_rt| {
                consumer(q_js_rt);
            });
            EventLoop::add_local_void(|| {
                QuickJsRuntimeAdapter::do_with(|q_js_rt| {
                    q_js_rt.run_pending_jobs_if_any();
                })
            })
        });
    }
}

/// EsRuntime is the main public struct representing a JavaScript runtime.
/// You can construct a new QuickJsRuntime by using the [QuickJsRuntimeBuilder] struct
/// # Example
/// ```rust
/// use quickjs_runtime::builder::QuickJsRuntimeBuilder;
/// let rt = QuickJsRuntimeBuilder::new().build();
/// ```
pub struct QuickJsRuntimeFacade {
    inner: Arc<QuickjsRuntimeFacadeInner>,
}

impl QuickJsRuntimeFacade {
    pub(crate) fn new(mut builder: QuickJsRuntimeBuilder) -> Self {
        let ret = Self {
            inner: Arc::new(QuickjsRuntimeFacadeInner {
                event_loop: EventLoop::new(),
            }),
        };

        ret.exe_task_in_event_loop(|| {
            let rt_ptr = unsafe { q::JS_NewRuntime() };
            let rt = QuickJsRuntimeAdapter::new(rt_ptr);
            QuickJsRuntimeAdapter::init_rt_for_current_thread(rt);
            functions::init_statics();
            reflection::init_statics();
        });

        // init ref in q_js_rt

        let rti_weak = Arc::downgrade(&ret.inner);

        ret.exe_task_in_event_loop(move || {
            QuickJsRuntimeAdapter::do_with_mut(move |m_q_js_rt| {
                m_q_js_rt.init_rti_ref(rti_weak);
            })
        });

        // run single job in eventQueue to init thread_local weak<rtref>

        #[cfg(any(
            feature = "settimeout",
            feature = "setinterval",
            feature = "console",
            feature = "setimmediate"
        ))]
        {
            let res = crate::features::init(&ret);
            if res.is_err() {
                panic!("could not init features: {}", res.err().unwrap());
            }
        }

        if let Some(interval) = builder.opt_gc_interval {
            let rti_ref: Weak<QuickjsRuntimeFacadeInner> = Arc::downgrade(&ret.inner);
            std::thread::spawn(move || loop {
                std::thread::sleep(interval);
                if let Some(el) = rti_ref.upgrade() {
                    log::debug!("running gc from gc interval thread");
                    el.event_loop.add_void(|| {
                        QJS_RT
                            .try_with(|rc| {
                                let rt = &*rc.borrow();
                                rt.as_ref().unwrap().gc();
                            })
                            .expect("QJS_RT.try_with failed");
                    });
                } else {
                    break;
                }
            });
        }

        let init_hooks: Vec<_> = builder.runtime_init_hooks.drain(..).collect();

        ret.exe_task_in_event_loop(move || {
            QuickJsRuntimeAdapter::do_with_mut(|q_js_rt| {
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
                for compiled_module_loader in builder.compiled_module_loaders {
                    q_js_rt.add_compiled_module_loader(CompiledModuleLoaderAdapter::new(
                        compiled_module_loader,
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

    /// get memory usage for this runtime
    pub async fn memory_usage(&self) -> MemoryUsage {
        self.loop_async(|rt| rt.memory_usage()).await
    }

    pub(crate) fn clear_contexts(&self) {
        log::trace!("EsRuntime::clear_contexts");
        self.exe_task_in_event_loop(|| {
            let context_ids = QuickJsRuntimeAdapter::get_context_ids();
            for id in context_ids {
                QuickJsRuntimeAdapter::remove_context(id.as_str());
            }
        });
    }

    /// this can be used to run a function in the event_queue thread for the QuickJSRuntime
    /// without borrowing the q_js_rt
    pub fn add_task_to_event_loop_void<C>(&self, task: C)
    where
        C: FnOnce() + Send + 'static,
    {
        self.inner.add_task_to_event_loop_void(task)
    }

    pub fn exe_task_in_event_loop<C, R: Send + 'static>(&self, task: C) -> R
    where
        C: FnOnce() -> R + Send + 'static,
    {
        self.inner.exe_task_in_event_loop(task)
    }

    pub fn add_task_to_event_loop<C, R: Send + 'static>(&self, task: C) -> impl Future<Output = R>
    where
        C: FnOnce() -> R + Send + 'static,
    {
        self.inner.add_task_to_event_loop(task)
    }

    /// this is how you add a closure to the worker thread which has an instance of the QuickJsRuntime
    /// this will run asynchronously
    /// # example
    /// ```rust
    /// use quickjs_runtime::builder::QuickJsRuntimeBuilder;
    /// let rt = QuickJsRuntimeBuilder::new().build();
    /// rt.add_rt_task_to_event_loop(|q_js_rt| {
    ///     // here you are in the worker thread and you can use the quickjs_utils
    ///     q_js_rt.gc();
    /// });
    /// ```
    pub fn add_rt_task_to_event_loop<C, R: Send + 'static>(
        &self,
        task: C,
    ) -> impl Future<Output = R>
    where
        C: FnOnce(&QuickJsRuntimeAdapter) -> R + Send + 'static,
    {
        self.inner.add_rt_task_to_event_loop(task)
    }

    pub fn add_rt_task_to_event_loop_void<C>(&self, task: C)
    where
        C: FnOnce(&QuickJsRuntimeAdapter) + Send + 'static,
    {
        self.inner.add_rt_task_to_event_loop_void(task)
    }

    /// used to add tasks from the worker threads which require run_pending_jobs_if_any to run after it
    #[allow(dead_code)]
    pub(crate) fn add_local_task_to_event_loop<C>(consumer: C)
    where
        C: FnOnce(&QuickJsRuntimeAdapter) + 'static,
    {
        QuickjsRuntimeFacadeInner::add_local_task_to_event_loop(consumer)
    }

    pub fn builder() -> QuickJsRuntimeBuilder {
        QuickJsRuntimeBuilder::new()
    }

    /// run the garbage collector asynchronously
    pub async fn gc(&self) {
        self.add_rt_task_to_event_loop(|q_js_rt| q_js_rt.gc()).await
    }

    /// run the garbage collector and wait for it to be done
    pub fn gc_sync(&self) {
        self.exe_rt_task_in_event_loop(|q_js_rt| q_js_rt.gc())
    }

    /// this is how you add a closure to the worker thread which has an instance of the QuickJsRuntime
    /// this will run and return synchronously
    /// # example
    /// ```rust
    /// use quickjs_runtime::builder::QuickJsRuntimeBuilder;
    /// use quickjs_runtime::jsutils::Script;
    /// use quickjs_runtime::quickjs_utils::primitives;
    /// let rt = QuickJsRuntimeBuilder::new().build();
    /// let res = rt.exe_rt_task_in_event_loop(|q_js_rt| {
    ///     let q_ctx = q_js_rt.get_main_realm();
    ///     // here you are in the worker thread and you can use the quickjs_utils
    ///     let val_ref = q_ctx.eval(Script::new("test.es", "(11 * 6);")).ok().expect("script failed");
    ///     primitives::to_i32(&val_ref).ok().expect("could not get i32")
    /// });
    /// assert_eq!(res, 66);
    /// ```
    pub fn exe_rt_task_in_event_loop<C, R>(&self, consumer: C) -> R
    where
        C: FnOnce(&QuickJsRuntimeAdapter) -> R + Send + 'static,
        R: Send + 'static,
    {
        self.exe_task_in_event_loop(|| QuickJsRuntimeAdapter::do_with(consumer))
    }

    /// this adds a rust function to JavaScript, it is added for all current and future contexts
    /// # Example
    /// ```rust
    /// use quickjs_runtime::builder::QuickJsRuntimeBuilder;
    /// use quickjs_runtime::quickjs_utils::primitives;
    /// use quickjs_runtime::jsutils::Script;
    /// use quickjs_runtime::values::{JsValueConvertable, JsValueFacade};
    ///  
    /// let rt = QuickJsRuntimeBuilder::new().build();
    ///
    /// rt.set_function(&["com", "mycompany", "util"], "methodA", |q_ctx, args: Vec<JsValueFacade>|{
    ///     let a = args[0].get_i32();
    ///     let b = args[1].get_i32();
    ///     Ok((a * b).to_js_value_facade())
    /// }).expect("set func failed");
    ///
    /// let res = rt.eval_sync(None, Script::new("test.es", "let a = com.mycompany.util.methodA(13, 17); a * 2;")).ok().expect("script failed");
    ///
    /// assert_eq!(res.get_i32(), (13*17*2));
    /// ```
    pub fn set_function<F>(
        &self,
        namespace: &[&str],
        name: &str,
        function: F,
    ) -> Result<(), JsError>
    where
        F: Fn(&QuickJsRealmAdapter, Vec<JsValueFacade>) -> Result<JsValueFacade, JsError>
            + Send
            + 'static,
    {
        let name = name.to_string();

        let namespace = namespace
            .iter()
            .map(|s| s.to_string())
            .collect::<Vec<String>>();

        self.exe_rt_task_in_event_loop(move |q_js_rt| {
            let func_rc = Rc::new(function);
            let name = name.to_string();

            q_js_rt.add_context_init_hook(move |_q_js_rt, realm| {
                let namespace_slice = namespace.iter().map(|s| s.as_str()).collect::<Vec<&str>>();
                let ns = objects::get_namespace_q(realm, &namespace_slice, true)?;

                let func_rc = func_rc.clone();

                let func = functions::new_function_q(
                    realm,
                    name.as_str(),
                    move |realm, _this_ref, args| {
                        let mut args_facades = vec![];

                        for arg_ref in args {
                            args_facades.push(realm.to_js_value_facade(arg_ref)?);
                        }

                        let res = func_rc(realm, args_facades);

                        match res {
                            Ok(val_jsvf) => realm.from_js_value_facade(val_jsvf),
                            Err(e) => Err(e),
                        }
                    },
                    1,
                )?;

                objects::set_property2_q(realm, &ns, name.as_str(), &func, 0)?;

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
    /// # Example
    /// ```
    /// use quickjs_runtime::builder::QuickJsRuntimeBuilder;
    /// use quickjs_runtime::jsutils::Script;
    /// let rt = QuickJsRuntimeBuilder::new().build();
    /// rt.create_context("my_context");
    /// rt.exe_rt_task_in_event_loop(|q_js_rt| {
    ///    let my_ctx = q_js_rt.get_context("my_context");
    ///    my_ctx.eval(Script::new("ctx_test.es", "this.myVar = 'only exists in my_context';"));
    /// });
    /// ```
    pub fn create_context(&self, id: &str) -> Result<(), JsError> {
        let id = id.to_string();
        self.inner
            .event_loop
            .exe(move || QuickJsRuntimeAdapter::create_context(id.as_str()))
    }

    /// drop a context which was created earlier with a call to [create_context()](struct.EsRuntime.html#method.create_context)
    pub fn drop_context(&self, id: &str) {
        let id = id.to_string();
        self.inner
            .event_loop
            .exe(move || QuickJsRuntimeAdapter::remove_context(id.as_str()))
    }
}

fn loop_realm_func<
    R: Send + 'static,
    C: FnOnce(&QuickJsRuntimeAdapter, &QuickJsRealmAdapter) -> R + Send + 'static,
>(
    realm_name: Option<String>,
    consumer: C,
) -> R {
    let res = QuickJsRuntimeAdapter::do_with(|q_js_rt| {
        if let Some(realm_str) = realm_name.as_ref() {
            if let Some(realm) = q_js_rt.get_realm(realm_str) {
                (Some(consumer(q_js_rt, realm)), None)
            } else {
                (None, Some(consumer))
            }
        } else {
            (Some(consumer(q_js_rt, q_js_rt.get_main_realm())), None)
        }
    });

    if let Some(res) = res.0 {
        res
    } else {
        // create realm first
        let consumer = res.1.unwrap();
        let realm_str = realm_name.expect("invalid state");

        QuickJsRuntimeAdapter::do_with_mut(|m_rt| {
            let ctx = QuickJsRealmAdapter::new(realm_str.to_string(), m_rt);
            m_rt.contexts.insert(realm_str.to_string(), ctx);
        });

        QuickJsRuntimeAdapter::do_with(|q_js_rt| {
            let realm = q_js_rt
                .get_realm(realm_str.as_str())
                .expect("invalid state");
            let hooks = &*q_js_rt.context_init_hooks.borrow();
            for hook in hooks {
                let res = hook(q_js_rt, realm);
                if res.is_err() {
                    panic!("realm init hook failed: {}", res.err().unwrap());
                }
            }

            consumer(q_js_rt, realm)
        })
    }
}

impl QuickJsRuntimeFacade {
    pub fn create_realm(&self, name: &str) -> Result<(), JsError> {
        let name = name.to_string();
        self.inner
            .event_loop
            .exe(move || QuickJsRuntimeAdapter::create_context(name.as_str()))
    }

    pub fn destroy_realm(&self, name: &str) -> Result<(), JsError> {
        let name = name.to_string();
        self.exe_task_in_event_loop(move || {
            QuickJsRuntimeAdapter::do_with_mut(|rt| {
                if rt.get_realm(name.as_str()).is_some() {
                    rt.remove_realm(name.as_str());
                }
                Ok(())
            })
        })
    }

    pub fn has_realm(&self, name: &str) -> Result<bool, JsError> {
        let name = name.to_string();
        self.exe_rt_task_in_event_loop(move |rt| Ok(rt.get_realm(name.as_str()).is_some()))
    }

    /// add a job to the eventloop which will execute sync(placed at end of eventloop)
    pub fn loop_sync<R: Send + 'static, C: FnOnce(&QuickJsRuntimeAdapter) -> R + Send + 'static>(
        &self,
        consumer: C,
    ) -> R {
        self.exe_rt_task_in_event_loop(consumer)
    }

    pub fn loop_sync_mut<
        R: Send + 'static,
        C: FnOnce(&mut QuickJsRuntimeAdapter) -> R + Send + 'static,
    >(
        &self,
        consumer: C,
    ) -> R {
        self.exe_task_in_event_loop(|| QuickJsRuntimeAdapter::do_with_mut(consumer))
    }

    /// add a job to the eventloop which will execute async(placed at end of eventloop)
    /// returns a Future which can be waited ob with .await
    pub fn loop_async<
        R: Send + 'static,
        C: FnOnce(&QuickJsRuntimeAdapter) -> R + Send + 'static,
    >(
        &self,
        consumer: C,
    ) -> Pin<Box<dyn Future<Output = R> + Send>> {
        Box::pin(self.add_rt_task_to_event_loop(consumer))
    }

    /// add a job to the eventloop (placed at end of eventloop) without expecting a result
    pub fn loop_void<C: FnOnce(&QuickJsRuntimeAdapter) + Send + 'static>(&self, consumer: C) {
        self.add_rt_task_to_event_loop_void(consumer)
    }

    /// add a job to the eventloop which will be executed synchronously (placed at end of eventloop)
    pub fn loop_realm_sync<
        R: Send + 'static,
        C: FnOnce(&QuickJsRuntimeAdapter, &QuickJsRealmAdapter) -> R + Send + 'static,
    >(
        &self,
        realm_name: Option<&str>,
        consumer: C,
    ) -> R {
        let realm_name = realm_name.map(|s| s.to_string());
        self.exe_task_in_event_loop(|| loop_realm_func(realm_name, consumer))
    }

    /// add a job to the eventloop which will be executed async (placed at end of eventloop)
    /// returns a Future which can be waited ob with .await
    pub fn loop_realm<
        R: Send + 'static,
        C: FnOnce(&QuickJsRuntimeAdapter, &QuickJsRealmAdapter) -> R + Send + 'static,
    >(
        &self,
        realm_name: Option<&str>,
        consumer: C,
    ) -> Pin<Box<dyn Future<Output = R>>> {
        let realm_name = realm_name.map(|s| s.to_string());
        Box::pin(self.add_task_to_event_loop(|| loop_realm_func(realm_name, consumer)))
    }

    /// add a job for a specific realm without expecting a result.
    /// the job will be added to the end of the eventloop
    pub fn loop_realm_void<
        C: FnOnce(&QuickJsRuntimeAdapter, &QuickJsRealmAdapter) + Send + 'static,
    >(
        &self,
        realm_name: Option<&str>,
        consumer: C,
    ) {
        let realm_name = realm_name.map(|s| s.to_string());
        self.add_task_to_event_loop_void(|| loop_realm_func(realm_name, consumer));
    }

    /// Evaluate a script asynchronously
    /// # Example
    /// ```rust
    /// use futures::executor::block_on;
    /// use quickjs_runtime::builder::QuickJsRuntimeBuilder;
    /// use quickjs_runtime::jsutils::Script;
    /// let rt = QuickJsRuntimeBuilder::new().build();
    /// let my_script = r#"
    ///    console.log("i'm a script");
    /// "#;
    /// block_on(rt.eval(None, Script::new("my_script.js", my_script))).expect("script failed");
    /// ```
    #[allow(clippy::type_complexity)]
    pub fn eval(
        &self,
        realm_name: Option<&str>,
        script: Script,
    ) -> Pin<Box<dyn Future<Output = Result<JsValueFacade, JsError>>>> {
        self.loop_realm(realm_name, |_rt, realm| {
            let res = realm.eval(script);
            match res {
                Ok(jsvr) => realm.to_js_value_facade(&jsvr),
                Err(e) => Err(e),
            }
        })
    }

    /// Evaluate a script and return the result synchronously
    /// # example
    /// ```rust
    /// use quickjs_runtime::builder::QuickJsRuntimeBuilder;
    /// use quickjs_runtime::jsutils::Script;
    /// let rt = QuickJsRuntimeBuilder::new().build();
    /// let script = Script::new("my_file.js", "(9 * 3);");
    /// let res = rt.eval_sync(None, script).ok().expect("script failed");
    /// assert_eq!(res.get_i32(), 27);
    /// ```
    #[allow(clippy::type_complexity)]
    pub fn eval_sync(
        &self,
        realm_name: Option<&str>,
        script: Script,
    ) -> Result<JsValueFacade, JsError> {
        self.loop_realm_sync(realm_name, |_rt, realm| {
            let res = realm.eval(script);
            match res {
                Ok(jsvr) => realm.to_js_value_facade(&jsvr),
                Err(e) => Err(e),
            }
        })
    }

    /// evaluate a module, you need this if you want to compile a script that contains static imports
    /// e.g.
    /// ```javascript
    /// import {util} from 'file.js';
    /// console.log(util(1, 2, 3));
    /// ```
    /// please note that the module is cached under the absolute path you passed in the Script object
    /// and thus you should take care to make the path unique (hence the absolute_ name)
    /// also to use this you need to build the QuickJsRuntimeFacade with a module loader
    /// # example
    /// ```rust
    /// use futures::executor::block_on;
    /// use quickjs_runtime::builder::QuickJsRuntimeBuilder;
    /// use quickjs_runtime::jsutils::modules::ScriptModuleLoader;
    /// use quickjs_runtime::jsutils::Script;
    /// use quickjs_runtime::quickjsrealmadapter::QuickJsRealmAdapter;
    /// struct TestModuleLoader {}
    /// impl ScriptModuleLoader for TestModuleLoader {
    ///     fn normalize_path(&self, _realm: &QuickJsRealmAdapter, ref_path: &str,path: &str) -> Option<String> {
    ///         Some(path.to_string())
    ///     }
    ///
    ///     fn load_module(&self, _realm: &QuickJsRealmAdapter, absolute_path: &str) -> String {
    ///         "export const util = function(a, b, c){return a+b+c;};".to_string()
    ///     }
    /// }
    /// let rt = QuickJsRuntimeBuilder::new().script_module_loader(TestModuleLoader{}).build();
    /// let script = Script::new("/opt/files/my_module.js", r#"
    ///     import {util} from 'other_module.js';\n
    ///     console.log(util(1, 2, 3));
    /// "#);
    /// // in real life you would .await this
    /// let _res = block_on(rt.eval_module(None, script));
    /// ```
    pub fn eval_module(
        &self,
        realm_name: Option<&str>,
        script: Script,
    ) -> Pin<Box<dyn Future<Output = Result<JsValueFacade, JsError>>>> {
        self.loop_realm(realm_name, |_rt, realm| {
            let res = realm.eval_module(script)?;
            realm.to_js_value_facade(&res)
        })
    }

    /// evaluate a module synchronously, you need this if you want to compile a script that contains static imports
    /// e.g.
    /// ```javascript
    /// import {util} from 'file.js';
    /// console.log(util(1, 2, 3));
    /// ```
    /// please note that the module is cached under the absolute path you passed in the Script object
    /// and thus you should take care to make the path unique (hence the absolute_ name)
    /// also to use this you need to build the QuickJsRuntimeFacade with a module loader
    /// # example
    /// ```rust
    /// use quickjs_runtime::builder::QuickJsRuntimeBuilder;
    /// use quickjs_runtime::jsutils::modules::ScriptModuleLoader;
    /// use quickjs_runtime::jsutils::Script;
    /// use quickjs_runtime::quickjsrealmadapter::QuickJsRealmAdapter;
    /// struct TestModuleLoader {}
    /// impl ScriptModuleLoader for TestModuleLoader {
    ///     fn normalize_path(&self, _realm: &QuickJsRealmAdapter, ref_path: &str,path: &str) -> Option<String> {
    ///         Some(path.to_string())
    ///     }
    ///
    ///     fn load_module(&self, _realm: &QuickJsRealmAdapter, absolute_path: &str) -> String {
    ///         "export const util = function(a, b, c){return a+b+c;};".to_string()
    ///     }
    /// }
    /// let rt = QuickJsRuntimeBuilder::new().script_module_loader(TestModuleLoader{}).build();
    /// let script = Script::new("/opt/files/my_module.js", r#"
    ///     import {util} from 'other_module.js';\n
    ///     console.log(util(1, 2, 3));
    /// "#);
    /// let _res = rt.eval_module_sync(None, script);
    /// ```
    pub fn eval_module_sync(
        &self,
        realm_name: Option<&str>,
        script: Script,
    ) -> Result<JsValueFacade, JsError> {
        self.loop_realm_sync(realm_name, |_rt, realm| {
            let res = realm.eval_module(script)?;
            realm.to_js_value_facade(&res)
        })
    }

    /// invoke a function in the engine and get the result synchronously
    /// # example
    /// ```rust
    /// use quickjs_runtime::builder::QuickJsRuntimeBuilder;
    /// use quickjs_runtime::jsutils::Script;
    /// use quickjs_runtime::values::JsValueConvertable;
    /// let rt = QuickJsRuntimeBuilder::new().build();
    /// let script = Script::new("my_file.es", "this.com = {my: {methodA: function(a, b, someStr, someBool){return a*b;}}};");
    /// rt.eval_sync(None, script).ok().expect("script failed");
    /// let res = rt.invoke_function_sync(None, &["com", "my"], "methodA", vec![7i32.to_js_value_facade(), 5i32.to_js_value_facade(), "abc".to_js_value_facade(), true.to_js_value_facade()]).ok().expect("func failed");
    /// assert_eq!(res.get_i32(), 35);
    /// ```
    #[warn(clippy::type_complexity)]
    pub fn invoke_function_sync(
        &self,
        realm_name: Option<&str>,
        namespace: &[&str],
        method_name: &str,
        args: Vec<JsValueFacade>,
    ) -> Result<JsValueFacade, JsError> {
        let movable_namespace: Vec<String> = namespace.iter().map(|s| s.to_string()).collect();
        let movable_method_name = method_name.to_string();

        self.loop_realm_sync(realm_name, move |_rt, realm| {
            let args_adapters: Vec<QuickJsValueAdapter> = args
                .into_iter()
                .map(|jsvf| realm.from_js_value_facade(jsvf).expect("conversion failed"))
                .collect();

            let namespace = movable_namespace
                .iter()
                .map(|s| s.as_str())
                .collect::<Vec<&str>>();

            let res = realm.invoke_function_by_name(
                namespace.as_slice(),
                movable_method_name.as_str(),
                args_adapters.as_slice(),
            );

            match res {
                Ok(jsvr) => realm.to_js_value_facade(&jsvr),
                Err(e) => Err(e),
            }
        })
    }

    /// invoke a function in the engine asynchronously
    /// N.B. func_name is not a &str because of <https://github.com/rust-lang/rust/issues/56238> (i think)
    /// # example
    /// ```rust
    /// use quickjs_runtime::builder::QuickJsRuntimeBuilder;
    /// use quickjs_runtime::jsutils::Script;
    /// use quickjs_runtime::values::JsValueConvertable;
    /// let rt = QuickJsRuntimeBuilder::new().build();
    /// let script = Script::new("my_file.es", "this.com = {my: {methodA: function(a, b){return a*b;}}};");
    /// rt.eval_sync(None, script).ok().expect("script failed");
    /// rt.invoke_function(None, &["com", "my"], "methodA", vec![7.to_js_value_facade(), 5.to_js_value_facade()]);
    /// ```
    #[allow(clippy::type_complexity)]
    pub fn invoke_function(
        &self,
        realm_name: Option<&str>,
        namespace: &[&str],
        method_name: &str,
        args: Vec<JsValueFacade>,
    ) -> Pin<Box<dyn Future<Output = Result<JsValueFacade, JsError>>>> {
        let movable_namespace: Vec<String> = namespace.iter().map(|s| s.to_string()).collect();
        let movable_method_name = method_name.to_string();

        self.loop_realm(realm_name, move |_rt, realm| {
            let args_adapters: Vec<QuickJsValueAdapter> = args
                .into_iter()
                .map(|jsvf| realm.from_js_value_facade(jsvf).expect("conversion failed"))
                .collect();

            let namespace = movable_namespace
                .iter()
                .map(|s| s.as_str())
                .collect::<Vec<&str>>();

            let res = realm.invoke_function_by_name(
                namespace.as_slice(),
                movable_method_name.as_str(),
                args_adapters.as_slice(),
            );

            match res {
                Ok(jsvr) => realm.to_js_value_facade(&jsvr),
                Err(e) => Err(e),
            }
        })
    }

    pub fn invoke_function_void(
        &self,
        realm_name: Option<&str>,
        namespace: &[&str],
        method_name: &str,
        args: Vec<JsValueFacade>,
    ) {
        let movable_namespace: Vec<String> = namespace.iter().map(|s| s.to_string()).collect();
        let movable_method_name = method_name.to_string();

        self.loop_realm_void(realm_name, move |_rt, realm| {
            let args_adapters: Vec<QuickJsValueAdapter> = args
                .into_iter()
                .map(|jsvf| realm.from_js_value_facade(jsvf).expect("conversion failed"))
                .collect();

            let namespace = movable_namespace
                .iter()
                .map(|s| s.as_str())
                .collect::<Vec<&str>>();

            let res = realm
                .invoke_function_by_name(
                    namespace.as_slice(),
                    movable_method_name.as_str(),
                    args_adapters.as_slice(),
                )
                .map(|jsvr| realm.to_js_value_facade(&jsvr));

            match res {
                Ok(_) => {
                    log::trace!(
                        "js_function_invoke_void succeeded: {}",
                        movable_method_name.as_str()
                    );
                }
                Err(err) => {
                    log::trace!(
                        "js_function_invoke_void failed: {}: {}",
                        movable_method_name.as_str(),
                        err
                    );
                }
            }
        })
    }
}

#[cfg(test)]
lazy_static! {
    static ref INITTED: std::sync::Mutex<bool> = std::sync::Mutex::new(false);
}

#[cfg(test)]
pub mod tests {

    use crate::facades::QuickJsRuntimeFacade;
    use crate::jsutils::modules::{NativeModuleLoader, ScriptModuleLoader};
    use crate::jsutils::JsError;
    use crate::jsutils::Script;
    use crate::quickjs_utils::{primitives, promises};
    use crate::quickjsrealmadapter::QuickJsRealmAdapter;
    use crate::quickjsvalueadapter::QuickJsValueAdapter;
    use crate::values::{JsValueConvertable, JsValueFacade};
    use backtrace::Backtrace;
    use futures::executor::block_on;
    use log::debug;
    use std::panic;
    use std::time::Duration;

    struct TestNativeModuleLoader {}
    struct TestScriptModuleLoader {}

    impl NativeModuleLoader for TestNativeModuleLoader {
        fn has_module(&self, _q_ctx: &QuickJsRealmAdapter, module_name: &str) -> bool {
            module_name.starts_with("greco://")
        }

        fn get_module_export_names(
            &self,
            _q_ctx: &QuickJsRealmAdapter,
            _module_name: &str,
        ) -> Vec<&str> {
            vec!["a", "b", "c"]
        }

        fn get_module_exports(
            &self,
            _q_ctx: &QuickJsRealmAdapter,
            _module_name: &str,
        ) -> Vec<(&str, QuickJsValueAdapter)> {
            vec![
                ("a", primitives::from_i32(1234)),
                ("b", primitives::from_i32(64834)),
                ("c", primitives::from_i32(333)),
            ]
        }
    }

    impl ScriptModuleLoader for TestScriptModuleLoader {
        fn normalize_path(
            &self,
            _realm: &QuickJsRealmAdapter,
            _ref_path: &str,
            path: &str,
        ) -> Option<String> {
            if path.eq("notfound.mes") || path.starts_with("greco://") {
                None
            } else {
                Some(path.to_string())
            }
        }

        fn load_module(&self, _realm: &QuickJsRealmAdapter, absolute_path: &str) -> String {
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

    #[test]
    pub fn test_stack_size() {
        let rt = init_test_rt();
        // 20 is ok, 200 fails
        let res = rt.eval_sync(
            None,
            Script::new(
                "stack_test.js",
                "let f = function(a){let f2 = arguments.callee; if (a < 20) {f2(a + 1);}}; f(1);",
            ),
        );
        match res {
            Ok(_) => {}
            Err(e) => {
                log::error!("fail: {}", e);
                panic!("fail: {}", e);
            }
        }

        let res = rt.eval_sync(
            None,
            Script::new(
                "stack_test.js",
                "let f = function(a){let f2 = arguments.callee; if (a < 1000) {f2(a + 1);}}; f(1);",
            ),
        );
        if res.is_ok() {
            panic!("stack should have overflowed");
        }
    }

    pub fn init_logging() {
        {
            let i_lock = &mut *crate::facades::INITTED.lock().unwrap();
            if !*i_lock {
                panic::set_hook(Box::new(|panic_info| {
                    let backtrace = Backtrace::new();
                    println!("thread panic occurred: {panic_info}\nbacktrace: {backtrace:?}");
                    log::error!(
                        "thread panic occurred: {}\nbacktrace: {:?}",
                        panic_info,
                        backtrace
                    );
                }));

                simple_logging::log_to_file("./quickjs_runtime.log", log::LevelFilter::max())
                    .expect("could not init logger");

                *i_lock = true;
            }
        }
    }

    pub fn init_test_rt() -> QuickJsRuntimeFacade {
        init_logging();

        QuickJsRuntimeFacade::builder()
            .gc_interval(Duration::from_secs(1))
            .max_stack_size(128 * 1024)
            .script_module_loader(TestScriptModuleLoader {})
            .native_module_loader(TestNativeModuleLoader {})
            .build()
    }

    #[test]
    fn test_func() {
        let rt = init_test_rt();
        let res = rt.set_function(&["nl", "my", "utils"], "methodA", |_q_ctx, args| {
            if args.len() != 2 || !args.first().unwrap().is_i32() || !args.get(1).unwrap().is_i32()
            {
                Err(JsError::new_str(
                    "i'd really like 2 args of the int32 kind please",
                ))
            } else {
                let a = args.first().unwrap().get_i32();
                let b = args.get(1).unwrap().get_i32();
                Ok((a * b).to_js_value_facade())
            }
        });

        match res {
            Ok(_) => {}
            Err(e) => {
                panic!("set_function failed: {}", e);
            }
        }

        let res = rt.eval_sync(
            None,
            Script::new("test_func.es", "(nl.my.utils.methodA(13, 56));"),
        );

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
        let rt = init_test_rt();
        let res = rt.eval_sync(None, Script::new("test.es", "console.log('foo bar');"));

        match res {
            Ok(_) => {}
            Err(e) => {
                panic!("eval failed: {}", e);
            }
        }

        let res = rt
            .eval_sync(None, Script::new("test.es", "(2 * 7);"))
            .expect("script failed");

        assert_eq!(res.get_i32(), 14);
    }

    #[test]
    fn t1234() {
        // test stack overflow
        let rt = init_test_rt();

        rt.exe_rt_task_in_event_loop(|q_js_rt| {
            //q_js_rt.run_pending_jobs_if_any();
            let q_ctx = q_js_rt.get_main_realm();
            let r = q_ctx.eval(Script::new(
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
        let rt = init_test_rt();

        let res = rt.eval_sync(None, Script::new(
            "test_async.es",
            "{let f = async function(){let p = new Promise((resolve, reject) => {resolve(12345);}); const p2 = await p; return p2}; f()};",
        ));

        match res {
            Ok(esvf) => {
                assert!(esvf.is_js_promise());
                match esvf {
                    JsValueFacade::JsPromise { cached_promise } => {
                        let p_res = cached_promise
                            .get_promise_result_sync()
                            .expect("promise timed out");
                        if p_res.is_err() {
                            panic!("{:?}", p_res.err().unwrap());
                        }
                        let res = p_res.ok().unwrap();
                        assert!(res.is_i32());
                        assert_eq!(res.get_i32(), 12345);
                    }
                    _ => {}
                }
            }
            Err(e) => {
                panic!("eval failed: {}", e);
            }
        }
    }

    #[test]
    fn test_promise() {
        let rt = init_test_rt();

        let res = rt.eval_sync(None, Script::new(
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
        let res: Result<JsValueFacade, JsError> = rt.eval_module_sync(
            None,
            Script::new(
                "test.es",
                "import {foo} from 'test_module.mes';\n console.log('static imp foo = ' + foo);",
            ),
        );

        match res {
            Ok(_) => {
                log::debug!("static import ok");
            }
            Err(e) => {
                log::error!("static import failed: {}", e);
            }
        }

        debug!("test dynamic import");
        let res: Result<JsValueFacade, JsError> = rt.eval_sync(None, Script::new(
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

        let a = rt
            .eval(None, Script::new("test_async.es", "122 + 1;"))
            .await;
        match a {
            Ok(a) => a.get_i32(),
            Err(e) => panic!("script failed: {}", e),
        }
    }

    #[test]
    fn test_async() {
        let fut = test_async1();
        let res = block_on(fut);
        assert_eq!(res, 123);
    }
}

#[cfg(test)]
pub mod abstraction_tests {
    use crate::builder::QuickJsRuntimeBuilder;
    use crate::facades::tests::init_test_rt;
    use crate::facades::QuickJsRuntimeFacade;
    use crate::jsutils::Script;
    use crate::values::JsValueFacade;
    use futures::executor::block_on;
    use serde::Deserialize;
    use serde::Serialize;

    async fn example(rt: &QuickJsRuntimeFacade) -> JsValueFacade {
        // add a job for the main realm (None as realm_name)
        rt.loop_realm(None, |_rt_adapter, realm_adapter| {
            let script = Script::new("example.js", "7 + 13");
            let value_adapter = realm_adapter.eval(script).expect("script failed");
            // convert value_adapter to value_facade because value_adapter is not Send
            realm_adapter
                .to_js_value_facade(&value_adapter)
                .expect("conversion failed")
        })
        .await
    }

    #[test]
    fn test1() {
        // start a new runtime
        let rt = QuickJsRuntimeBuilder::new().build();
        let val = block_on(example(&rt));
        if let JsValueFacade::I32 { val } = val {
            assert_eq!(val, 20);
        } else {
            panic!("not an i32");
        }
    }

    #[tokio::test]
    async fn test_serde() {
        let json = r#"
            {
                "a": 1,
                "b": true,
                "c": {
                    "d": "q",
                    "e": [1, 2, 3.3]
                }
            }
        "#;

        let value = serde_json::from_str::<serde_json::Value>(json).expect("json fail");
        let input: JsValueFacade = JsValueFacade::SerdeValue { value };
        let rt = init_test_rt();

        let _ = rt.eval(None, Script::new("t.js", r#"
            function testSerde(input) {
                return "" + input.a + input.b + input.c.d + input.c.e[0] + input.c.e[1] + input.c.e[2];
            }
        "#)).await.expect("script failed");

        let res = rt
            .invoke_function(None, &[], "testSerde", vec![input])
            .await
            .expect("func failed");

        assert!(res.is_string());
        assert_eq!(res.get_str(), "1trueq123.3");
    }

    #[derive(Serialize, Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct User {
        name: String,
        last_name: String,
    }

    #[tokio::test]
    async fn serde_tests_serialize() {
        let rtb: QuickJsRuntimeBuilder = QuickJsRuntimeBuilder::new();
        let rt = rtb.build();

        // init my function
        rt.eval(
            None,
            Script::new(
                "test.js",
                r#"
                function myTest(user) {
                    return {
                        name: "proc_" + user.name,
                        lastName: "proc_" + user.lastName
                    }
                }
                "#,
            ),
        )
        .await
        .expect("script failed");

        // create a user obj
        let test_user_input = User {
            last_name: "Anderson".to_string(),
            name: "Mister".to_string(),
        };

        let args = vec![JsValueFacade::from_serializable(&test_user_input)
            .expect("could not serialize to JsValueFacade")];

        let res: JsValueFacade = rt
            .invoke_function(None, &[], "myTest", args)
            .await
            .expect("func failed");

        let json_result = res
            .to_json_string()
            .await
            .expect("could not serialize to json");

        assert_eq!(
            json_result.as_str(),
            r#"{"name":"proc_Mister","lastName":"proc_Anderson"}"#
        );

        // serialize back to user
        let user_output: User = serde_json::from_str(json_result.as_str()).unwrap();
        assert_eq!(user_output.name.as_str(), "proc_Mister");
        assert_eq!(user_output.last_name.as_str(), "proc_Anderson");
    }

    #[tokio::test]
    async fn serde_tests_value() {
        let rtb: QuickJsRuntimeBuilder = QuickJsRuntimeBuilder::new();
        let rt = rtb.build();

        // init my function
        rt.eval(
            None,
            Script::new(
                "test.js",
                r#"
                function myTest(user) {
                    return {
                        name: "proc_" + user.name,
                        lastName: "proc_" + user.lastName
                    }
                }
                "#,
            ),
        )
        .await
        .expect("script failed");

        // create a user obj
        let test_user_input = User {
            last_name: "Anderson".to_string(),
            name: "Mister".to_string(),
        };

        let input_value: serde_json::Value =
            serde_json::to_value(test_user_input).expect("could not to_value");
        let args = vec![JsValueFacade::SerdeValue { value: input_value }];

        let res: JsValueFacade = rt
            .invoke_function(None, &[], "myTest", args)
            .await
            .expect("func failed");

        // as value
        let value_result: serde_json::Value = res
            .to_serde_value()
            .await
            .expect("could not serialize to json");

        assert!(value_result.is_object());

        // serialize back to user
        let user_output: User = serde_json::from_value(value_result).unwrap();
        assert_eq!(user_output.name.as_str(), "proc_Mister");
        assert_eq!(user_output.last_name.as_str(), "proc_Anderson");
    }
}
