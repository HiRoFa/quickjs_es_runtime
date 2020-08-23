use quick_js::console::{ConsoleBackend, Level};
use quick_js::JsValue;

pub(crate) struct QuickJsConsole {}
impl ConsoleBackend for QuickJsConsole {
    fn log(&self, level: Level, values: Vec<JsValue>) {
        for val in values {
            log::info!("console: {}, {}", level, val.as_str().unwrap());
        }
    }
}
