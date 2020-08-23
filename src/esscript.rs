pub struct EsScript {
    absolute_path: String,
    script_code: String,
}

impl EsScript {
    pub fn new(absolute_path: String, script_code: String) -> Self {
        Self {
            absolute_path,
            script_code,
        }
    }
    pub fn get_path(&self) -> &str {
        self.absolute_path.as_str()
    }
    pub fn get_code(&self) -> &str {
        self.script_code.as_str()
    }
}
