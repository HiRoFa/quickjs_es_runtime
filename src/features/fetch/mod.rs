use crate::eserror::EsError;
use crate::esruntime::EsRuntime;
use crate::esruntime_utils::promises;
use crate::features::fetch::request::FetchRequest;
use crate::features::fetch::response::FetchResponse;
use crate::quickjs_utils;
use crate::quickjs_utils::{functions, objects};
use crate::quickjsruntime::QuickJsRuntime;
use crate::valueref::JSValueRef;
use libquickjs_sys as q;
use std::collections::HashMap;
use std::sync::Arc;

pub mod request;
pub mod response;

pub(crate) fn init(es_rt: Arc<EsRuntime>) -> Result<(), EsError> {
    es_rt.add_to_event_queue_sync(|q_js_rt| {
        log::trace!("fetch::init");

        // init the fetch method

        let func_ref =
            functions::new_native_function(q_js_rt, "fetch", Some(fetch_func), 1, false)?;
        objects::set_property(
            q_js_rt,
            &quickjs_utils::get_global(q_js_rt),
            "fetch",
            func_ref,
        )?;

        response::init_response_proxy(q_js_rt);

        Ok(())
    })
}

unsafe extern "C" fn fetch_func(
    _ctx: *mut q::JSContext,
    _this_val: q::JSValue,
    argc: ::std::os::raw::c_int,
    argv: *mut q::JSValue,
) -> q::JSValue {
    let arg_slice = std::slice::from_raw_parts(argv, argc as usize);
    let _args_vec: Vec<JSValueRef> = arg_slice
        .iter()
        .map(|raw| JSValueRef::new(*raw, false, false, "fetch_func_arg"))
        .collect::<Vec<_>>();

    QuickJsRuntime::do_with(|q_js_rt| {
        if let Some(rt_ref) = q_js_rt.get_rt_ref() {
            if rt_ref.inner.fetch_response_provider.is_some() {
                let producer = move || {
                    // call fetch_result_producer()

                    // we are out of thread here, so we should get a ref to es_rt before and move it here
                    // hmm so how do we use the fetch_provider out of the q_js_rt thread... i gues the provider should only be part of es_rt?

                    let provider = rt_ref
                        .inner
                        .fetch_response_provider
                        .as_ref()
                        .expect("we really expected a fetch_response_provider here");

                    let request = FetchRequest::new("", HashMap::new());

                    let result: Box<dyn FetchResponse + Send> = provider(&request);

                    Ok(result)
                };
                let mapper =
                    |p_res: Box<dyn FetchResponse + Send>| response::new_response_ref(p_res);
                let es_rt = &*q_js_rt.get_rt_ref().unwrap();
                let prom_res = promises::new_resolving_promise(q_js_rt, producer, mapper, es_rt);
                match prom_res {
                    Ok(prom_ref) => prom_ref.consume_value_no_decr_rc(),
                    Err(e) => q_js_rt.report_ex(e.get_message()),
                }
            } else {
                q_js_rt.report_ex("No fetch response provider present in this runtime")
            }
        } else {
            q_js_rt.report_ex("Runtime was dropped")
        }
    })
}

#[cfg(test)]
pub mod tests {

    use crate::esruntime::EsRuntime;
    use crate::esruntimebuilder::EsRuntimeBuilder;
    use crate::esscript::EsScript;
    use crate::features::fetch::FetchResponse;
    use std::sync::Arc;
    use std::time::Duration;

    struct TestResponse {
        txt: Option<String>,
    }
    impl FetchResponse for TestResponse {
        fn get_http_status(&self) -> u16 {
            unimplemented!()
        }

        fn get_header(&self, _name: &str) -> &[&str] {
            unimplemented!()
        }

        fn get_header_names(&self) -> &[&String] {
            unimplemented!()
        }

        fn read(&mut self) -> Option<Vec<u8>> {
            if self.txt.is_some() {
                let s = std::mem::replace(&mut self.txt, None);
                Some(s.unwrap().into_bytes())
            } else {
                None
            }
        }
    }

    #[test]
    fn test_fetch() {
        let _: Arc<EsRuntime> = crate::esruntime::tests::TEST_ESRT.clone();
        let rt = EsRuntimeBuilder::new()
            .fetch_response_provider(|_req| {
                let res = TestResponse {
                    txt: Some("{\"test\": \"response\"}".to_string()),
                };
                Box::new(res)
            })
            .build();
        let res = rt.eval_sync(EsScript::new(
            "test_fetch.es",
            "let res = fetch('https://httpbin.org/get'); console.log('fetch res was: ' + res); res.then((fetch_resp) => {console.log('fetch response .ok = ' + fetch_resp.ok); fetch_resp.text().then((txt) => {console.log('fetch_resp.text() resolved into ' + txt);});}); res = null;",
        ));
        match res {
            Ok(_) => {
                //
            }
            Err(e) => {
                panic!("script failed: {}", e);
            }
        }
        let res2 = rt.eval_sync(EsScript::new(
            "test_fetch2.es",
            "let res2 = fetch('https://httpbin.org/get'); console.log('fetch res2 was: ' + res2); res2.then((fetch_resp) => {console.log('fetch response .ok = ' + fetch_resp.ok); fetch_resp.json().then((js_obj) => {console.log('fetch_resp.json() resolved into ' + js_obj);}).catch((ex) => {console.log('fetch_resp.caught ' + ex);});;}); res2 = null;",
        ));
        match res2 {
            Ok(_) => {
                //
            }
            Err(e) => {
                panic!("script2 failed: {}", e);
            }
        }
        std::thread::sleep(Duration::from_secs(2));
        rt.gc_sync();
        std::thread::sleep(Duration::from_secs(2));
        rt.eval_sync(EsScript::new("test_fetch2.es", "1+1;"))
            .ok()
            .expect("foo");
        std::thread::sleep(Duration::from_secs(2));
    }
}
