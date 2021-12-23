//! contains the QuickJsRuntimeBuilder which may be used to instantiate a new QuickjsRuntimeFacade

use crate::facades::QuickJsRuntimeFacade;
use crate::quickjsrealmadapter::QuickJsRealmAdapter;
use crate::quickjsruntimeadapter::QuickJsRuntimeAdapter;
use hirofa_utils::js_utils::adapters::JsRuntimeAdapter;
use hirofa_utils::js_utils::facades::{JsRuntimeBuilder, JsRuntimeFacade};
use hirofa_utils::js_utils::modules::{
    CompiledModuleLoader, NativeModuleLoader, ScriptModuleLoader,
};
use hirofa_utils::js_utils::JsError;
use hirofa_utils::js_utils::ScriptPreProcessor;
use std::time::Duration;

pub type EsRuntimeInitHooks =
    Vec<Box<dyn FnOnce(&QuickJsRuntimeFacade) -> Result<(), JsError> + Send + 'static>>;

/// the EsRuntimeBuilder is used to init an EsRuntime
/// # Example
/// ```rust
/// use quickjs_runtime::builder::QuickJsRuntimeBuilder;
/// // init a rt which may use 16MB of memory
/// let rt = QuickJsRuntimeBuilder::new()
/// .memory_limit(1024*1024*16)
/// .build();
/// ```
pub struct QuickJsRuntimeBuilder {
    pub(crate) script_module_loaders: Vec<Box<dyn ScriptModuleLoader<QuickJsRealmAdapter> + Send>>,
    pub(crate) native_module_loaders: Vec<Box<dyn NativeModuleLoader<QuickJsRealmAdapter> + Send>>,
    pub(crate) compiled_module_loaders:
        Vec<Box<dyn CompiledModuleLoader<QuickJsRealmAdapter> + Send>>,
    pub(crate) opt_memory_limit_bytes: Option<u64>,
    pub(crate) opt_gc_threshold: Option<u64>,
    pub(crate) opt_max_stack_size: Option<u64>,
    pub(crate) opt_gc_interval: Option<Duration>,
    pub(crate) runtime_init_hooks: EsRuntimeInitHooks,
    pub(crate) script_pre_processors: Vec<Box<dyn ScriptPreProcessor + Send>>,
    pub(crate) interrupt_handler: Option<Box<dyn Fn(&QuickJsRuntimeAdapter) -> bool + Send>>,
}

impl QuickJsRuntimeBuilder {
    /// build an EsRuntime
    pub fn build(self) -> QuickJsRuntimeFacade {
        QuickJsRuntimeFacade::new(self)
    }

    /// init a new EsRuntimeBuilder
    pub fn new() -> Self {
        Self {
            script_module_loaders: vec![],
            native_module_loaders: vec![],
            compiled_module_loaders: vec![],
            opt_memory_limit_bytes: None,
            opt_gc_threshold: None,
            opt_max_stack_size: None,
            opt_gc_interval: None,
            runtime_init_hooks: vec![],
            script_pre_processors: vec![],
            interrupt_handler: None,
        }
    }

    /// add a script loaders which will be used to load modules when they are imported from script
    /// # Example
    /// ```rust
    /// use hirofa_utils::js_utils::Script;
    /// use quickjs_runtime::builder::QuickJsRuntimeBuilder;
    /// use quickjs_runtime::quickjsrealmadapter::QuickJsRealmAdapter;
    /// use hirofa_utils::js_utils::modules::ScriptModuleLoader;
    /// struct MyModuleLoader {}
    /// impl ScriptModuleLoader<QuickJsRealmAdapter> for MyModuleLoader {
    ///     fn normalize_path(&self, realm: &QuickJsRealmAdapter ,ref_path: &str,path: &str) -> Option<String> {
    ///         Some(path.to_string())
    ///     }
    ///
    ///     fn load_module(&self, realm: &QuickJsRealmAdapter, absolute_path: &str) -> String {
    ///         "export const foo = 12;".to_string()
    ///     }
    /// }
    ///
    /// let rt = QuickJsRuntimeBuilder::new()
    ///     .script_module_loader(Box::new(MyModuleLoader{}))
    ///     .build();
    /// rt.eval_module_sync(Script::new("test_module.es", "import {foo} from 'some_module.mes';\nconsole.log('foo = %s', foo);")).ok().unwrap();
    /// ```
    pub fn script_module_loader<M: ScriptModuleLoader<QuickJsRealmAdapter> + Send + 'static>(
        mut self,
        loader: Box<M>,
    ) -> Self {
        self.script_module_loaders.push(loader);
        self
    }

    /// add a ScriptPreProcessor which will be called for all scripts which are evaluated and compiled
    pub fn script_pre_processor<S: ScriptPreProcessor + Send + 'static>(
        mut self,
        processor: S,
    ) -> Self {
        self.script_pre_processors.push(Box::new(processor));
        self
    }

    pub fn runtime_init_hook<H>(mut self, hook: H) -> Self
    where
        H: FnOnce(&QuickJsRuntimeFacade) -> Result<(), JsError> + Send + 'static,
    {
        self.runtime_init_hooks.push(Box::new(hook));
        self
    }

    /// add a module loader which can load native functions and proxy classes
    /// # Example
    /// ```rust
    /// use quickjs_runtime::builder::QuickJsRuntimeBuilder;
    /// use quickjs_runtime::valueref::JSValueRef;
    /// use quickjs_runtime::quickjsrealmadapter::QuickJsRealmAdapter;
    /// use quickjs_runtime::quickjs_utils::functions;
    /// use quickjs_runtime::quickjs_utils::primitives::{from_bool, from_i32};
    /// use quickjs_runtime::reflection::Proxy;
    /// use hirofa_utils::js_utils::Script;
    /// use hirofa_utils::js_utils::modules::NativeModuleLoader;
    ///
    /// struct MyModuleLoader{}
    /// impl NativeModuleLoader<QuickJsRealmAdapter> for MyModuleLoader {
    ///     fn has_module(&self, _q_ctx: &QuickJsRealmAdapter,module_name: &str) -> bool {
    ///         module_name.eq("my_module")
    ///     }
    ///
    ///     fn get_module_export_names(&self, _q_ctx: &QuickJsRealmAdapter, _module_name: &str) -> Vec<&str> {
    ///         vec!["someVal", "someFunc", "SomeClass"]
    ///     }
    ///
    ///     fn get_module_exports(&self, q_ctx: &QuickJsRealmAdapter, _module_name: &str) -> Vec<(&str, JSValueRef)> {
    ///         
    ///         let js_val = from_i32(1470);
    ///         let js_func = functions::new_function_q(
    ///             q_ctx,
    ///             "someFunc", |_q_ctx, _this, _args| {
    ///                 return Ok(from_i32(432));
    ///             }, 0)
    ///             .ok().unwrap();
    ///         let js_class = Proxy::new()
    ///             .name("SomeClass")
    ///             .static_method("doIt", |_q_ctx, _args|{
    ///                 return Ok(from_i32(185));
    ///             })
    ///             .install(q_ctx, false)
    ///             .ok().unwrap();
    ///
    ///         vec![("someVal", js_val), ("someFunc", js_func), ("SomeClass", js_class)]
    ///     }
    /// }
    ///
    /// let rt = QuickJsRuntimeBuilder::new()
    /// .native_module_loader(Box::new(MyModuleLoader{}))
    /// .build();
    ///
    /// rt.eval_module_sync(Script::new("test_native_mod.es", "import {someVal, someFunc, SomeClass} from 'my_module';\nlet i = (someVal + someFunc() + SomeClass.doIt());\nif (i !== 2087){throw Error('i was not 2087');}")).ok().expect("script failed");
    /// ```
    pub fn native_module_loader<M: NativeModuleLoader<QuickJsRealmAdapter> + Send + 'static>(
        mut self,
        loader: Box<M>,
    ) -> Self {
        self.native_module_loaders.push(loader);
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

    /// set a Garbage Collection interval, this will start a timer thread which will trigger a full GC every set interval
    pub fn gc_interval(mut self, interval: Duration) -> Self {
        self.opt_gc_interval = Some(interval);
        self
    }

    /// add an interrupt handler, this will be called several times during script execution and may be used to cancel a running script
    pub fn set_interrupt_handler<I: Fn(&QuickJsRuntimeAdapter) -> bool + Send + 'static>(
        mut self,
        interrupt_handler: I,
    ) -> Self {
        self.interrupt_handler = Some(Box::new(interrupt_handler));
        self
    }
}

impl Default for QuickJsRuntimeBuilder {
    fn default() -> Self {
        QuickJsRuntimeBuilder::new()
    }
}

impl JsRuntimeBuilder for QuickJsRuntimeBuilder {
    type JsRuntimeFacadeType = QuickJsRuntimeFacade;

    fn js_build(self) -> QuickJsRuntimeFacade {
        self.build()
    }

    fn js_runtime_init_hook<
        H: FnOnce(&QuickJsRuntimeFacade) -> Result<(), JsError> + Send + 'static,
    >(
        mut self,
        hook: H,
    ) -> Self {
        self.runtime_init_hooks.push(Box::new(hook));
        self
    }

    fn js_realm_adapter_init_hook<H: FnOnce(&<<Self as JsRuntimeBuilder>::JsRuntimeFacadeType as JsRuntimeFacade>::JsRuntimeAdapterType, &<<<Self as JsRuntimeBuilder>::JsRuntimeFacadeType as JsRuntimeFacade>::JsRuntimeAdapterType as JsRuntimeAdapter>::JsRealmAdapterType) -> Result<(), JsError> + Send + 'static>(self, _hook: H) -> Self{
        todo!()
    }

    fn js_runtime_adapter_init_hook<H: FnOnce(&<<Self as JsRuntimeBuilder>::JsRuntimeFacadeType as JsRuntimeFacade>::JsRuntimeAdapterType) -> Result<(), JsError> + Send + 'static>(self, _hook: H) -> Self{
        todo!()
    }

    fn js_script_pre_processor<S: ScriptPreProcessor + Send + 'static>(
        mut self,
        preprocessor: S,
    ) -> Self {
        self.script_pre_processors.push(Box::new(preprocessor));
        self
    }

    fn js_script_module_loader<S: ScriptModuleLoader<QuickJsRealmAdapter> + Send + 'static>(
        mut self,
        module_loader: S,
    ) -> Self {
        self.script_module_loaders.push(Box::new(module_loader));
        self
    }

    fn js_compiled_module_loader<
        S: CompiledModuleLoader<<<<Self as JsRuntimeBuilder>::JsRuntimeFacadeType as JsRuntimeFacade>::JsRuntimeAdapterType as JsRuntimeAdapter>::JsRealmAdapterType>
        + Send
        + 'static
    >(
        mut self,
        module_loader: S,
    ) -> Self{
        self.compiled_module_loaders.push(Box::new(module_loader));
        self
    }

    fn js_native_module_loader<
        S: NativeModuleLoader<<<<Self as JsRuntimeBuilder>::JsRuntimeFacadeType as JsRuntimeFacade>::JsRuntimeAdapterType as JsRuntimeAdapter>::JsRealmAdapterType>
        + Send
        + 'static,
    >(mut self, module_loader: S) -> Self where
    Self: Sized{
        self.native_module_loaders.push(Box::new(module_loader));
        self
    }
}

#[cfg(test)]
pub mod tests {
    use crate::builder::QuickJsRuntimeBuilder;
    use crate::quickjsrealmadapter::QuickJsRealmAdapter;
    use hirofa_utils::js_utils::modules::ScriptModuleLoader;
    use hirofa_utils::js_utils::Script;

    #[test]
    fn test_module_loader() {
        struct MyModuleLoader {}
        impl ScriptModuleLoader<QuickJsRealmAdapter> for MyModuleLoader {
            fn normalize_path(
                &self,
                _realm: &QuickJsRealmAdapter,
                _ref_path: &str,
                path: &str,
            ) -> Option<String> {
                Some(path.to_string())
            }

            fn load_module(&self, _realm: &QuickJsRealmAdapter, _absolute_path: &str) -> String {
                "export const foo = 12;".to_string()
            }
        }

        let rt = QuickJsRuntimeBuilder::new()
            .script_module_loader(Box::new(MyModuleLoader {}))
            .build();
        match rt.eval_module_sync(Script::new(
            "test_module.es",
            "import {foo} from 'some_module.mes';\nconsole.log('foo = %s', foo);",
        )) {
            Ok(_) => {}
            Err(e) => panic!("script failed {}", e),
        }
    }
}
