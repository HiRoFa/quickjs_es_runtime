use std::fmt::{Error, Formatter};

pub struct EsError {
    name: String,
    message: String,
    stack: String,
}

impl EsError {
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
        EsError {
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

impl std::fmt::Display for EsError {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), Error> {
        let e = format!("{}: {} at{}", self.name, self.message, self.stack);
        f.write_str(e.as_str())
    }
}

#[cfg(test)]
pub mod tests {
    use crate::esruntime::EsRuntime;
    use crate::esscript::EsScript;
    use crate::quickjs_utils::functions;
    use std::sync::Arc;
    use std::time::Duration;

    #[test]
    fn test_err() {
        let rt: Arc<EsRuntime> = crate::esruntime::tests::TEST_ESRT.clone();
        rt.add_to_event_queue_sync(|q_js_rt| {
            let q_ctx = q_js_rt.get_main_context();
            let f = q_ctx
                .eval(EsScript::new(
                    "err.js",
                    "(function err(){throw Error('Oh dear, stuff failed');});",
                ))
                .ok()
                .unwrap();

            let f_res = functions::call_function_q(q_ctx, &f, vec![], None);
            assert!(f_res.is_err());
            let err = format!("{}", f_res.err().unwrap());
            //let _l = err.len();
            //log::info!("err.len={}", l);
            //log::info!("err = {}", err);
            if !err.contains("Oh dear") {
                panic!("sdf, err was {}", err);
            }
            //panic!("sdf, err was {}", err);
        });
        std::thread::sleep(Duration::from_secs(1));
    }
}
