use std::collections::HashMap;

pub struct FetchRequest {
    url: String,
    headers: HashMap<String, Vec<String>>,
}

impl FetchRequest {
    pub fn new(url: &str, headers: HashMap<String, Vec<String>>) -> Self {
        Self {
            url: url.to_string(),
            headers,
        }
    }
    pub fn get_url(&self) -> &str {
        self.url.as_str()
    }
    pub fn get_header(&self, name: &str) -> &[String] {
        self.headers.get(name).unwrap().as_slice()
    }
}
