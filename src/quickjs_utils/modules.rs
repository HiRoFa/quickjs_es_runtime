//! utils for working with ES6 Modules

use crate::eserror::EsError;
use crate::esscript::EsScript;
use crate::quickjs_utils::atoms;
use crate::quickjs_utils::atoms::JSAtomRef;
use crate::quickjscontext::QuickJsContext;
use crate::quickjsruntime::QuickJsRuntime;
use crate::valueref::JSValueRef;
use libquickjs_sys as q;
use log::trace;
use std::cell::RefCell;
use std::collections::HashSet;
use std::ffi::{CStr, CString};
use std::os::raw::c_int;
use std::ptr;

thread_local! {
    // todo refactor to per ctx (map should be instance of quickjscontext instead of thread_local)
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

/// detect if a script is module (contains import or export statements)
pub fn detect_module(source: &str) -> bool {
    let cstr = CString::new(source).expect("could not create CString due to null term in source");
    unsafe { q::JS_DetectModule(cstr.as_ptr(), source.len() as _) != 0 }
}

/// create new Module (JSModuleDef struct) which can be populated with exports after (and from) the init_func
/// # Safety
/// Please ensure the context passed is still valid
pub unsafe fn new_module(
    ctx: *mut q::JSContext,
    name: &str,
    init_func: q::JSModuleInitFunc,
) -> Result<*mut q::JSModuleDef, EsError> {
    let name_cstr = CString::new(name).map_err(|_e| EsError::new_str("CString failed"))?;
    Ok(q::JS_NewCModule(ctx, name_cstr.as_ptr(), init_func))
}

/// set an export in a JSModuleDef, this should be called AFTER the init_func(as passed to new_module()) is called
/// please note that you always need to use this in combination with add_module_export()
/// # Safety
/// Please ensure the context passed is still valid
pub unsafe fn set_module_export(
    ctx: *mut q::JSContext,
    module: *mut q::JSModuleDef,
    export_name: &str,
    js_val: JSValueRef,
) -> Result<(), EsError> {
    let name_cstr = CString::new(export_name).map_err(|_e| EsError::new_str("CString failed"))?;
    let res = q::JS_SetModuleExport(
        ctx,
        module,
        name_cstr.as_ptr(),
        js_val.clone_value_incr_rc(),
    );
    if res == 0 {
        Ok(())
    } else {
        Err(EsError::new_str("JS_SetModuleExport failed"))
    }
}

/// set an export in a JSModuleDef, this should be called BEFORE this init_func(as passed to new_module()) is called
/// # Safety
/// Please ensure the context passed is still valid
pub unsafe fn add_module_export(
    ctx: *mut q::JSContext,
    module: *mut q::JSModuleDef,
    export_name: &str,
) -> Result<(), EsError> {
    let name_cstr = CString::new(export_name).map_err(|_e| EsError::new_str("CString failed"))?;
    let res = q::JS_AddModuleExport(ctx, module, name_cstr.as_ptr());
    if res == 0 {
        Ok(())
    } else {
        Err(EsError::new_str("JS_SetModuleExport failed"))
    }
}

/// get the name of an JSModuleDef struct
/// # Safety
/// Please ensure the context passed is still valid
pub unsafe fn get_module_name(
    ctx: *mut q::JSContext,
    module: *mut q::JSModuleDef,
) -> Result<String, EsError> {
    let atom_raw = q::JS_GetModuleName(ctx, module);
    let atom_ref = JSAtomRef::new(ctx, atom_raw);
    atoms::to_string(ctx, &atom_ref)
}

unsafe extern "C" fn js_module_normalize(
    ctx: *mut q::JSContext,
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
        let q_ctx = q_js_rt.get_quickjs_context(ctx);

        if let Some(loader) = &q_js_rt.module_script_loader {
            loader(q_ctx, base_str, name_str)
        } else {
            None
        }
    });

    let mut absolute_path = name_str.to_string();

    if let Some(script) = script_opt {
        absolute_path = script.get_path().to_string();
        // todo, refactor this to per ctx, or drop entirely?
        let needs_init = LOADED_MODULE_REGISTRY.with(|registry_rc| {
            let registry = &*registry_rc.borrow();
            !registry.contains(&absolute_path)
        });
        if needs_init {
            trace!("module {} not loaded, initializing", name_str);

            // init module here
            let eval_res = QuickJsContext::eval_module_ctx(ctx, script);
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
                    // report_ex_ctx
                    QuickJsContext::report_ex_ctx(
                        ctx,
                        format!("Module eval failed for {}\ncaused by {}", name_str, e).as_str(),
                    );
                    return ptr::null_mut();
                }
            }
        } else {
            trace!("module {} was already loaded, doing nothing", name_str);
        }
    } else {
        trace!("no module found for {} at {}", name_str, base_str);

        QuickJsContext::report_ex_ctx(ctx, format!("Module {} was not found", name_str).as_str());
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

unsafe extern "C" fn native_module_init(
    ctx: *mut q::JSContext,
    module: *mut q::JSModuleDef,
) -> c_int {
    let module_name = get_module_name(ctx, module)
        .ok()
        .expect("could not get name");
    log::trace!("native_module_init: {}", module_name);

    QuickJsRuntime::do_with(|q_js_rt| {
        if let Some(module_loader) = &q_js_rt.native_module_loader {
            QuickJsContext::with_context(ctx, |q_ctx| {
                for (name, val) in module_loader.get_module_exports(q_ctx, module_name.as_str()) {
                    set_module_export(ctx, module, name, val)
                        .ok()
                        .expect("could not set export");
                }
            })
        }
    });

    0 // ok
}

unsafe extern "C" fn js_module_loader(
    ctx: *mut q::JSContext,
    module_name_raw: *const ::std::os::raw::c_char,
    _opaque: *mut ::std::os::raw::c_void,
) -> *mut q::JSModuleDef {
    log::trace!("js_module_loader called.");

    let module_name_c = CStr::from_ptr(module_name_raw);
    let module_name = module_name_c.to_str().expect("could not get module name");

    log::trace!("js_module_loader called: {}", module_name);

    QuickJsRuntime::do_with(|q_js_rt| {
        if let Some(module_loader) = &q_js_rt.native_module_loader {
            QuickJsContext::with_context(ctx, |q_ctx| {
                if module_loader.has_module(q_ctx, module_name) {
                    let module = new_module(ctx, module_name, Some(native_module_init))
                        .ok()
                        .expect("could not create new module");

                    for name in module_loader.get_module_export_names(q_ctx, module_name) {
                        add_module_export(ctx, module, name)
                            .ok()
                            .expect("could not add export");
                    }

                    //std::ptr::null_mut()
                    module
                } else {
                    std::ptr::null_mut()
                }
            })
        } else {
            std::ptr::null_mut()
        }
    })
}

#[cfg(test)]
pub mod tests {
    use crate::esruntime::EsRuntime;
    use crate::esscript::EsScript;
    use crate::quickjs_utils::modules::detect_module;
    use std::sync::Arc;
    use std::time::Duration;

    #[test]
    fn test_native_modules() {
        let rt: Arc<EsRuntime> = crate::esruntime::tests::TEST_ESRT.clone();
        let mres = rt.eval_module_sync(EsScript::new(
            "test.mes",
            "import {a, b, c} from 'greco://testmodule1';\nconsole.log('testmodule1.a = %s, testmodule1.b = %s, testmodule1.c = %s', a, b, c);",
        ));
        match mres {
            Ok(_module_res) => {}
            Err(e) => panic!("test_native_modules failed: {}", e),
        }
    }

    #[test]
    fn test_detect() {
        assert!(detect_module("import {} from 'foo.es';"));
        assert!(detect_module("export function a(){};"));
        assert!(!detect_module("import('foo.es').then((a) = {});"));
        assert!(!detect_module("let a = 1;"));
    }

    #[test]
    fn test_module_sandbox() {
        log::info!("> test_module_sandbox");

        let rt: Arc<EsRuntime> = crate::esruntime::tests::TEST_ESRT.clone();
        rt.add_to_event_queue_sync(|q_js_rt| {
            let q_ctx = q_js_rt.get_main_context();
            let res = q_ctx.eval_module(EsScript::new(
                "test1.mes",
                "export const name = 'foobar';\nconsole.log('evalling module');",
            ));

            if res.is_err() {
                panic!("parse module failed: {}", res.err().unwrap())
            }
            res.ok().expect("parse module failed");
        });

        rt.add_to_event_queue_sync(|q_js_rt| {
            let q_ctx = q_js_rt.get_main_context();
            let res = q_ctx.eval_module(EsScript::new(
                "test2.mes",
                "import {name} from 'test1.mes';\n\nconsole.log('imported name: ' + name);",
            ));

            if res.is_err() {
                panic!("parse module2 failed: {}", res.err().unwrap())
            }

            res.ok().expect("parse module2 failed");
        });

        rt.add_to_event_queue_sync(|q_js_rt| {
            let q_ctx = q_js_rt.get_main_context();
            let res = q_ctx.eval_module(EsScript::new(
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
            let q_ctx = q_js_rt.get_main_context();
            let res = q_ctx.eval_module(EsScript::new(
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
            let q_ctx = q_js_rt.get_main_context();
            let res = q_ctx.eval_module(EsScript::new(
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
