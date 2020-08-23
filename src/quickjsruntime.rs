// store in thread_local

use crate::esscript::EsScript;
use crate::quickjsconsole::QuickJsConsole;
use quick_js::{Context, ExecutionError, JsValue};
use std::cell::RefCell;

thread_local! {
   /// the thread-local SpiderMonkeyRuntime
   /// this only exists for the worker thread of the EsEventQueue
   pub(crate) static QJS_RT: RefCell<QuickJsRuntime> = RefCell::new(QuickJsRuntime::new());
}

pub struct QuickJsRuntime {
    context: Context,
}

impl QuickJsRuntime {
    fn new() -> Self {
        let console = QuickJsConsole {};
        Self {
            context: Context::builder().console(console).build().unwrap(),
        }
    }

    pub fn gc(&self) {}

    pub fn eval(&self, script: EsScript) -> Result<JsValue, ExecutionError> {
        self.context.eval(script.get_code())
    }

    pub(crate) fn do_with<C, R>(task: C) -> R
    where
        C: FnOnce(&QuickJsRuntime) -> R + Send + 'static,
        R: Send + 'static,
    {
        QJS_RT.with(|qjs_rc| {
            let qjs_rt = &*qjs_rc.borrow();
            task(qjs_rt)
        })
    }
}
