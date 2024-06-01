//! This contains abstract traits and structs for use with different javascript runtimes
//! the Adapter traits are use in the worker thread (EventLoop) of the Runtime and thus are not Send, they should never leave the thread
//! The facade classes are for use outside the worker thread, they are Send
//!

use backtrace::Backtrace;
use std::fmt::{Debug, Display, Error, Formatter};

pub mod helper_tasks;
pub mod jsproxies;
pub mod modules;
pub mod promises;

pub trait ScriptPreProcessor {
    fn process(&self, script: &mut Script) -> Result<(), JsError>;
}

/// the JsValueType represents the type of value for a JSValue
#[derive(PartialEq, Copy, Clone, Eq)]
pub enum JsValueType {
    I32,
    F64,
    String,
    Boolean,
    Object,
    Function,
    BigInt,
    Promise,
    Date,
    Null,
    Undefined,
    Array,
    Error,
}

impl Display for JsValueType {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            JsValueType::I32 => f.write_str("I32"),
            JsValueType::F64 => f.write_str("F64"),
            JsValueType::String => f.write_str("String"),
            JsValueType::Boolean => f.write_str("Boolean"),
            JsValueType::Object => f.write_str("Object"),
            JsValueType::Function => f.write_str("Function"),
            JsValueType::BigInt => f.write_str("BigInt"),
            JsValueType::Promise => f.write_str("Promise"),
            JsValueType::Date => f.write_str("Date"),
            JsValueType::Null => f.write_str("Null"),
            JsValueType::Undefined => f.write_str("Undefined"),
            JsValueType::Array => f.write_str("Array"),
            JsValueType::Error => f.write_str("Error"),
        }
    }
}

#[derive(Debug)]
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
        let bt = Backtrace::new();
        JsError {
            name: "Error".to_string(),
            message: err,
            stack: format!("{bt:?}"),
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

impl std::error::Error for JsError {
    fn description(&self) -> &str {
        self.get_message()
    }
}

impl std::fmt::Display for JsError {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), Error> {
        let e = format!("{}: {}\n{}", self.name, self.message, self.stack);
        f.write_str(e.as_str())
    }
}

impl From<Error> for JsError {
    fn from(e: Error) -> Self {
        JsError::new_string(format!("{e}"))
    }
}

pub struct Script {
    path: String,
    code: String,
    transpiled_code: Option<String>,
    map: Option<String>,
}

impl Debug for Script {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str(format!("Script:{}", self.path.as_str()).as_str())
    }
}

impl Script {
    pub fn new(absolute_path: &str, script_code: &str) -> Self {
        Self {
            path: absolute_path.to_string(),
            code: script_code.to_string(),
            transpiled_code: None,
            map: None,
        }
    }
    pub fn get_path(&self) -> &str {
        self.path.as_str()
    }
    pub fn get_code(&self) -> &str {
        self.code.as_str()
    }
    pub fn get_runnable_code(&self) -> &str {
        if let Some(t_code) = self.transpiled_code.as_ref() {
            t_code.as_str()
        } else {
            self.code.as_str()
        }
    }
    pub fn set_code(&mut self, code: String) {
        self.code = code;
    }
    pub fn set_transpiled_code(&mut self, transpiled_code: String, map: Option<String>) {
        self.transpiled_code = Some(transpiled_code);
        self.map = map;
    }
    pub fn get_map(&self) -> Option<&str> {
        self.map.as_deref()
    }
}

impl Clone for Script {
    fn clone(&self) -> Self {
        Self {
            path: self.path.clone(),
            code: self.code.clone(),
            transpiled_code: self.transpiled_code.clone(),
            map: self.map.clone(),
        }
    }
}
