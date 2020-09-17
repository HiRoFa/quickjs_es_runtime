pub struct EsScript {
    absolute_path: String,
    script_code: String,
}

impl EsScript {
    pub fn new(absolute_path: &str, script_code: &str) -> Self {
        Self {
            absolute_path: absolute_path.to_string(),
            script_code: script_code.to_string(),
        }
    }
    pub fn get_path(&self) -> &str {
        self.absolute_path.as_str()
    }
    pub fn get_code(&self) -> &str {
        self.script_code.as_str()
    }
}
