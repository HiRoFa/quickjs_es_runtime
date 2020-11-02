use crate::eserror::EsError;
use crate::esruntime_utils::promises::new_resolving_promise;
use crate::quickjs_utils::{json, primitives, reflection};
use crate::quickjsruntime::QuickJsRuntime;
use crate::valueref::JSValueRef;
use std::cell::RefCell;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

type FetchResponseType = Box<dyn FetchResponse + Send>;
type FetchResponseMapType = Arc<Mutex<FetchResponseType>>;

thread_local! {
    static RESPONSES : RefCell<HashMap<usize, FetchResponseMapType>> = RefCell::new(HashMap::new());
}

pub trait FetchResponse {
    fn get_http_status(&self) -> u16;
    fn get_header(&self, name: &str) -> Option<&str>;
    fn read(&mut self) -> Option<Vec<u8>>;
}

const RESPONSE_PROXY_NAME: &str = "Response";

fn response_text(
    _q_js_rt: &QuickJsRuntime,
    instance_id: &usize,
    _args: Vec<JSValueRef>,
) -> Result<JSValueRef, EsError> {
    QuickJsRuntime::do_with(|q_js_rt| {
        let es_rt_arc_opt = q_js_rt.get_rt_ref();
        let es_rt = &*es_rt_arc_opt.unwrap();

        let resp_arc: FetchResponseMapType = RESPONSES.with(move |rrc| {
            let responses_map = &*rrc.borrow();
            responses_map
                .get(&instance_id)
                .expect("no such response found")
                .clone()
        });

        let producer = move || {
            // get response, read till completion, return full str
            let fr_mtx = &*resp_arc;
            let fr = &mut *fr_mtx.lock().unwrap();
            let mut bytes = vec![];
            while let Some(mut buffer) = fr.read() {
                bytes.append(&mut buffer);
            }

            let res_str = String::from_utf8(bytes);
            match res_str {
                Ok(s) => Ok(s),
                Err(_e) => Err("UTF8Error while reading text".to_string()),
            }
        };
        let mapper = |res: String| {
            // map string to js_str
            QuickJsRuntime::do_with(|q_js_rt| primitives::from_string(q_js_rt, res.as_str()))
        };

        new_resolving_promise(q_js_rt, producer, mapper, &es_rt)
    })
}

fn response_json(
    _q_js_rt: &QuickJsRuntime,
    instance_id: &usize,
    _args: Vec<JSValueRef>,
) -> Result<JSValueRef, EsError> {
    QuickJsRuntime::do_with(|q_js_rt| {
        let es_rt_arc_opt = q_js_rt.get_rt_ref();
        let es_rt = &*es_rt_arc_opt.unwrap();

        let resp_arc: FetchResponseMapType = RESPONSES.with(move |rrc| {
            let responses_map = &*rrc.borrow();
            responses_map
                .get(&instance_id)
                .expect("no such response found")
                .clone()
        });

        let producer = move || {
            // get response, read till completion, return full str
            let fr_mtx = &*resp_arc;
            let fr = &mut *fr_mtx.lock().unwrap();
            let mut bytes = vec![];
            while let Some(mut buffer) = fr.read() {
                bytes.append(&mut buffer);
            }

            let res_str = String::from_utf8(bytes);
            match res_str {
                Ok(s) => Ok(s),
                Err(_e) => Err("UTF8Error while reading text".to_string()),
            }
        };
        let mapper = |res: String| {
            // map string to js_str and then parse
            QuickJsRuntime::do_with(|q_js_rt| {
                //
                let str_ref = primitives::from_string(q_js_rt, res.as_str())?;

                let s = primitives::to_string(q_js_rt, &str_ref)?;

                json::parse(q_js_rt, s.as_str())
            })
        };

        new_resolving_promise(q_js_rt, producer, mapper, &es_rt)
    })
}

pub(crate) fn init_response_proxy(q_js_rt: &QuickJsRuntime) {
    reflection::Proxy::new()
        .name(RESPONSE_PROXY_NAME)
        // todo native_methods
        .method("text", response_text)
        .method("json", response_json)
        .getter_setter(
            "headers",
            //todo Headers proxy obj
            |_q_js_rt, _instance_id| Ok(primitives::from_bool(true)),
            |_q_js_rt, _instance_id, _val| Ok(()),
        )
        .getter_setter(
            "ok",
            //todo
            |_q_js_rt, _instance_id| Ok(primitives::from_bool(true)),
            |_q_js_rt, _instance_id, _val| Ok(()),
        )
        .getter_setter(
            "status",
            // todo
            |_q_js_rt, _instance_id| Ok(primitives::from_i32(200)),
            |_q_js_rt, _instance_id, _val| Ok(()),
        )
        .finalizer(|instance_id| {
            log::trace!("dropping FetchResponse {}", instance_id);
            RESPONSES.with(|responses_rc| {
                let responses = &mut *responses_rc.borrow_mut();
                responses.remove(&instance_id);
            });
        })
        .install(q_js_rt)
        .ok()
        .expect("could not install Response proxy");
}

pub(crate) fn new_response_ref(
    fetch_response: Box<dyn FetchResponse + Send>,
) -> Result<JSValueRef, EsError> {
    QuickJsRuntime::do_with(|q_js_rt| {
        let res = reflection::new_instance(RESPONSE_PROXY_NAME, q_js_rt)?;

        RESPONSES.with(|responses_rc| {
            let responses = &mut *responses_rc.borrow_mut();
            responses.insert(res.0, Arc::new(Mutex::new(fetch_response)))
        });

        Ok(res.1)
    })
}
