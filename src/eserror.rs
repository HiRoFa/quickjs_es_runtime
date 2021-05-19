use hirofa_utils::js_utils::JsError;
use std::fmt::{Error, Formatter};

/// The JsError struct is used throughout this crate to represent errors

pub struct JsError {
    name: String,
    message: String,
    stack: String,
}

impl JsError {
    pub fn new(name: String, message: String, stack: String) -> Self {
        Self {
            name,
            message,
            stack,
        }
    }
    pub fn new_str(err: &str) -> Self {
        Self::new_string(err.to_string())
    }
    pub fn new_string(err: String) -> Self {
        JsError {
            name: "".to_string(),
            message: err,
            stack: "".to_string(),
        }
    }
    pub fn get_message(&self) -> &str {
        self.message.as_str()
    }
    pub fn get_stack(&self) -> &str {
        self.stack.as_str()
    }
    pub fn get_name(&self) -> &str {
        self.name.as_str()
    }
}

impl std::fmt::Display for JsError {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), Error> {
        let e = format!("{}: {} at{}", self.name, self.message, self.stack);
        f.write_str(e.as_str())
    }
}
