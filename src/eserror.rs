use std::fmt::{Formatter, Error};

pub struct EsError {
    message: String,
    file_name: String,
    line_no: u32,
    col_no: u32,
}

impl EsError {
    pub fn new_str(err: &str) -> Self {
        Self::new_string(err.to_string())
    }
    pub fn new_string(err: String) -> Self {
        EsError {
            message: err,
            file_name: "".to_string(),
            line_no: 0,
            col_no: 0
        }
    }
}

impl std::fmt::Display for EsError {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), Error> {
        let e = format!("error in {} at {}:{} err: {}", self.file_name, self.line_no, self.col_no, self.message);
        f.write_str(e.as_str())
    }
}

