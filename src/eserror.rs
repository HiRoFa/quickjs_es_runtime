use std::fmt::{Error, Formatter};

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
            col_no: 0,
        }
    }
    pub fn get_message(&self) -> &str {
        self.message.as_str()
    }
    pub fn get_file_name(&self) -> &str {
        self.file_name.as_str()
    }
    pub fn get_col(&self) -> u32 {
        self.col_no
    }
    pub fn get_line(&self) -> u32 {
        self.line_no
    }
}

impl std::fmt::Display for EsError {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), Error> {
        if self.file_name.is_empty() {
            f.write_str(self.message.as_str())
        } else {
            let e = format!(
                "{}:{}:{} error message: {}",
                self.file_name, self.line_no, self.col_no, self.message
            );
            f.write_str(e.as_str())
        }
    }
}
