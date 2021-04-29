use crate::eserror::EsError;
use crate::esruntime::EsRuntime;
use crate::esruntime_utils::promises;
use crate::features::fetch::request::FetchRequest;
use crate::features::fetch::response::FetchResponse;
use crate::quickjs_utils;
use crate::quickjs_utils::{functions, objects, parse_args, primitives};
use crate::quickjscontext::QuickJsContext;
use crate::quickjsruntime::QuickJsRuntime;
use libquickjs_sys as q;
use std::collections::HashMap;
use std::sync::Arc;

pub mod request;
pub mod response;

pub(crate) fn init(es_rt: &EsRuntime) -> Result<(), EsError> {
    es_rt.exe_rt_task_in_event_loop(|q_js_rt| {
        q_js_rt.add_context_init_hook(|_q_js_rt, q_ctx| {
            log::trace!("fetch::init");

            // init the fetch method

            let func_ref =
                functions::new_native_function_q(q_ctx, "fetch", Some(fetch_func), 2, false)?;
            unsafe {
                objects::set_property(
                    q_ctx.context,
                    &quickjs_utils::get_global(q_ctx.context),
                    "fetch",
                    &func_ref,
                )
            }?;

            response::init_response_proxy(q_ctx)
        })
    })
}

unsafe extern "C" fn fetch_func(
    ctx: *mut q::JSContext,
    _this_val: q::JSValue,
    argc: ::std::os::raw::c_int,
    argv: *mut q::JSValue,
) -> q::JSValue {
    let args_vec = parse_args(ctx, argc, argv);

    QuickJsRuntime::do_with(|q_js_rt| {
        let q_ctx = q_js_rt.get_quickjs_context(ctx);
        if args_vec.is_empty() {
            return q_ctx.report_ex("need at least a url arg");
        }

        let url_arg = &args_vec[0];
        if !url_arg.is_string() {
            return q_ctx.report_ex("url argument needs to be a string");
        }

        let url = primitives::to_string(ctx, &url_arg).ok().unwrap();

        if let Some(rt_ref) = q_js_rt.get_rt_ref() {
            if rt_ref.get_fetch_response_provider().is_some() {
                let rt_ref_weak = Arc::downgrade(&rt_ref);
                // prevent accidental use
                drop(rt_ref);

                let producer = move || {
                    // call fetch_result_producer()

                    if let Some(rt_ref) = rt_ref_weak.upgrade() {
                        let provider = rt_ref
                            .get_fetch_response_provider()
                            .expect("we really expected a fetch_response_provider here");

                        let request = FetchRequest::new(url.as_str(), HashMap::new());

                        let result: Box<dyn FetchResponse + Send> = provider(&request);

                        Ok(result)
                    } else {
                        Err("rt was dropped".to_string())
                    }
                };
                let mapper = |q_ctx: &QuickJsContext, p_res: Box<dyn FetchResponse + Send>| {
                    response::new_response_ref(q_ctx, p_res)
                };
                let es_rt = q_js_rt.get_rt_ref().unwrap();

                let prom_res = promises::new_resolving_promise(q_ctx, producer, mapper, es_rt);
                match prom_res {
                    Ok(prom_ref) => prom_ref.clone_value_incr_rc(),
                    Err(e) => q_ctx.report_ex(e.get_message()),
                }
            } else {
                q_ctx.report_ex("No fetch response provider present in this runtime")
            }
        } else {
            q_ctx.report_ex("Runtime was dropped")
        }
    })
}

#[cfg(test)]
pub mod tests {

    use crate::esruntime::tests::init_test_rt;
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

        fn get_header(&self, _name: &str) -> Option<&str> {
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
        let main_rt: Arc<EsRuntime> = init_test_rt();

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
            "{let res = fetch('https://httpbin.org/get'); console.log('fetch res was: ' + res); res.then((fetch_resp) => {console.log('fetch response .ok = ' + fetch_resp.ok); fetch_resp.text().then((txt) => {console.log('fetch_resp.text() resolved into ' + txt);});}); res = null;}",
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
            "{let res2 = fetch('https://httpbin.org/get'); console.log('fetch res2 was: ' + res2); res2.then((fetch_resp) => {console.log('fetch response .ok = ' + fetch_resp.ok); fetch_resp.json().then((js_obj) => {console.log('fetch_resp.json() resolved into ' + js_obj);}).catch((ex) => {console.log('fetch_resp.caught ' + ex);});;}); res2 = null;}",
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

        log::trace!("test_fetch rt.gc");
        rt.gc_sync();
        log::trace!("test_fetch main_rt.gc");
        main_rt.gc_sync();
        log::trace!("test_fetch drop rt");
        drop(rt);
        log::trace!("test_fetch sleep");
        std::thread::sleep(Duration::from_secs(2));
    }
}
