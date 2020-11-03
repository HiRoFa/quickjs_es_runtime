use crate::esscript::EsScript;
use crate::quickjsruntime::QuickJsRuntime;
use libquickjs_sys as q;
use log::trace;
use std::cell::RefCell;
use std::collections::HashSet;
use std::ffi::{CStr, CString};

thread_local! {
    static LOADED_MODULE_REGISTRY: RefCell<HashSet<String>> = RefCell::new(HashSet::new());
}

#[allow(dead_code)]
pub fn set_module_loader(q_js_rt: &QuickJsRuntime) {
    log::trace!("setting up module loader");

    let module_normalize: q::JSModuleNormalizeFunc = Some(js_module_normalize);
    let module_loader: q::JSModuleLoaderFunc = Some(js_module_loader);

    let opaque = std::ptr::null_mut();

    unsafe { q::JS_SetModuleLoaderFunc(q_js_rt.runtime, module_normalize, module_loader, opaque) }
}

unsafe extern "C" fn js_module_normalize(
    _ctx: *mut q::JSContext,
    module_base_name: *const ::std::os::raw::c_char,
    module_name: *const ::std::os::raw::c_char,
    _opaque: *mut ::std::os::raw::c_void,
) -> *mut ::std::os::raw::c_char {
    log::trace!("js_module_normalize called.");

    let base_c = CStr::from_ptr(module_base_name);
    let base_str = base_c
        .to_str()
        .expect("could not convert module_base_name to str");
    let name_c = CStr::from_ptr(module_name);
    let name_str = name_c
        .to_str()
        .expect("could not convert module_name to str");

    log::trace!(
        "js_module_normalize called. base: {}. name: {}",
        base_str,
        name_str
    );

    let script_opt: Option<EsScript> = QuickJsRuntime::do_with(|q_js_rt| {
        if let Some(loader) = &q_js_rt.module_script_loader {
            loader(base_str, name_str)
        } else {
            None
        }
    });

    let mut absolute_path = name_str.to_string();

    if let Some(script) = script_opt {
        absolute_path = script.get_path().to_string();
        let needs_init = LOADED_MODULE_REGISTRY.with(|registry_rc| {
            let registry = &*registry_rc.borrow();
            !registry.contains(&absolute_path)
        });
        if needs_init {
            trace!("module {} not loaded, initializing", name_str);

            // init module here
            let eval_res = QuickJsRuntime::do_with(|q_js_rt| q_js_rt.eval_module(script));
            match eval_res {
                Ok(_) => {
                    // add to registry
                    trace!("module {} was loaded, adding to registry", name_str);
                    LOADED_MODULE_REGISTRY.with(|registry_rc| {
                        let registry = &mut *registry_rc.borrow_mut();
                        registry.insert(absolute_path.to_string())
                    });
                }
                Err(e) => {
                    log::error!("module {} failed: {}", name_str, e);
                    QuickJsRuntime::do_with(|q_js_rt| {
                        q_js_rt.report_ex(
                            format!("Module eval failed for {}\ncaused by {}", name_str, e)
                                .as_str(),
                        )
                    });
                }
            }
        } else {
            trace!("module {} was already loaded, doing nothing", name_str);
        }
    } else {
        trace!("no module found for {} at {}", name_str, base_str);
        QuickJsRuntime::do_with(|q_js_rt| {
            let _ = q_js_rt.report_ex(format!("Module {} was not found", name_str).as_str());
        });
    }

    let c_absolute_path_res = CString::new(absolute_path.as_str());
    match c_absolute_path_res {
        Ok(c_absolute_path) => c_absolute_path.into_raw(),
        Err(_e) => {
            log::error!(
                "could not normalize due to NullError about: {}",
                absolute_path.as_str()
            );
            panic!("could not normalize due to NullError");
        }
    }
}

unsafe extern "C" fn js_module_loader(
    _ctx: *mut q::JSContext,
    module_name: *const ::std::os::raw::c_char,
    _opaque: *mut ::std::os::raw::c_void,
) -> *mut q::JSModuleDef {
    log::trace!("js_module_loader called.");

    let module_name_c = CStr::from_ptr(module_name);
    let res = module_name_c.to_str();

    if res.is_err() {
        panic!("failed to get module name: {}", res.err().unwrap());
    }

    log::trace!(
        "js_module_loader called: {}",
        res.expect("could not get module_name")
    );

    std::ptr::null_mut()
}

#[cfg(test)]
pub mod tests {
    use crate::esruntime::EsRuntime;
    use crate::esscript::EsScript;
    use std::sync::Arc;
    use std::time::Duration;

    #[test]
    fn test_module_sandbox() {
        log::info!("> test_module_sandbox");

        let rt: Arc<EsRuntime> = crate::esruntime::tests::TEST_ESRT.clone();
        rt.add_to_event_queue_sync(|q_js_rt| {
            let res = q_js_rt.eval_module(EsScript::new(
                "test1.mes",
                "export const name = 'foobar';\nconsole.log('evalling module');",
            ));

            if res.is_err() {
                panic!("parse module failed: {}", res.err().unwrap())
            }
            res.ok().expect("parse module failed");
        });

        rt.add_to_event_queue_sync(|q_js_rt| {
            let res = q_js_rt.eval_module(EsScript::new(
                "test2.mes",
                "import {name} from 'test1.mes';\n\nconsole.log('imported name: ' + name);",
            ));

            if res.is_err() {
                panic!("parse module2 failed: {}", res.err().unwrap())
            }

            res.ok().expect("parse module2 failed");
        });

        rt.add_to_event_queue_sync(|q_js_rt| {
            let res = q_js_rt.eval_module(EsScript::new(
                "test3.mes",
                "import {name} from 'notfound.mes';\n\nconsole.log('imported name: ' + name);",
            ));

            assert!(res.is_err());
            assert!(res
                .err()
                .unwrap()
                .get_message()
                .contains("Module notfound.mes was not found"));
        });

        rt.add_to_event_queue_sync(|q_js_rt| {
            let res = q_js_rt.eval_module(EsScript::new(
                "test4.mes",
                "import {name} from 'invalid.mes';\n\nconsole.log('imported name: ' + name);",
            ));

            assert!(res.is_err());
            assert!(res
                .err()
                .unwrap()
                .get_message()
                .contains("Module eval failed for invalid.mes"));
        });

        rt.add_to_event_queue_sync(|q_js_rt| {
            let res = q_js_rt.eval_module(EsScript::new(
                "test2.mes",
                "import {name} from 'test1.mes';\n\nconsole.log('imported name: ' + name);",
            ));

            if res.is_err() {
                panic!("parse module2 failed: {}", res.err().unwrap())
            }

            res.ok().expect("parse module2 failed");
        });

        std::thread::sleep(Duration::from_secs(1));

        log::info!("< test_module_sandbox");
    }
}
