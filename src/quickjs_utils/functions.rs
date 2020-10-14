use crate::eserror::EsError;
use crate::quickjs_utils::{atoms, errors, objects, primitives};
use crate::quickjsruntime::{make_cstring, QuickJsRuntime};
use crate::valueref::JSValueRef;
use hirofa_utils::auto_id_map::AutoIdMap;
use libquickjs_sys as q;
use log::trace;
use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::os::raw::{c_char, c_void};

#[allow(dead_code)]
pub fn call_function(
    q_js_rt: &QuickJsRuntime,
    function_ref: &JSValueRef,
    arguments: Vec<JSValueRef>,
    this_ref_opt: Option<&JSValueRef>,
) -> Result<JSValueRef, EsError> {
    log::trace!("functions::call_function()");

    assert!(is_function(q_js_rt, function_ref));

    let arg_count = arguments.len() as i32;

    let mut qargs = arguments
        .iter()
        .map(|a| *a.borrow_value())
        .collect::<Vec<_>>();

    let this_val = match this_ref_opt {
        Some(r) => *r.borrow_value(),
        None => crate::quickjs_utils::new_null(),
    };

    let res = unsafe {
        q::JS_Call(
            q_js_rt.context,
            *function_ref.borrow_value(),
            this_val,
            arg_count,
            qargs.as_mut_ptr(),
        )
    };

    let mut res_ref = JSValueRef::new(res, false, true, "call_function result");
    res_ref.label("functions::call_function res");

    if res_ref.is_exception() {
        if let Some(ex) = q_js_rt.get_exception() {
            Err(ex)
        } else {
            Err(EsError::new_str(
                "function invocation failed but could not get ex",
            ))
        }
    } else {
        Ok(res_ref)
    }
}

/*
pub fn JS_Invoke(
        ctx: *mut JSContext,
        this_val: JSValue,
        atom: JSAtom,
        argc: ::std::os::raw::c_int,
        argv: *mut JSValue,
    ) -> JSValue;
 */

#[allow(dead_code)]
pub fn invoke_member_function(
    q_js_rt: &QuickJsRuntime,
    obj_ref: &JSValueRef,
    function_name: &str,
    arguments: Vec<JSValueRef>,
) -> Result<JSValueRef, EsError> {
    let arg_count = arguments.len() as i32;

    let atm = atoms::from_string(q_js_rt, function_name)?;

    let mut qargs = arguments
        .iter()
        .map(|a| *a.borrow_value())
        .collect::<Vec<_>>();

    let res_val = unsafe {
        q::JS_Invoke(
            q_js_rt.context,
            *obj_ref.borrow_value(),
            atm,
            arg_count,
            qargs.as_mut_ptr(),
        )
    };

    let res_ref = JSValueRef::new(
        res_val,
        false,
        true,
        format!("functions::invoke_member_function res: {}", function_name).as_str(),
    );

    Ok(res_ref)
}

pub fn call_to_string(q_js_rt: &QuickJsRuntime, obj_ref: &JSValueRef) -> Result<String, EsError> {
    if obj_ref.is_string() {
        crate::quickjs_utils::primitives::to_string(q_js_rt, obj_ref)
    } else {
        log::trace!("calling JS_ToString on a {}", obj_ref.borrow_value().tag);

        let res = unsafe { q::JS_ToString(q_js_rt.context, *obj_ref.borrow_value()) };
        let res_ref = JSValueRef::new(res, false, true, "call_to_string result");

        log::trace!("called JS_ToString got a {}", res_ref.borrow_value().tag);

        if !res_ref.is_string() {
            return Err(EsError::new_str("Could not convert value to string"));
        }
        crate::quickjs_utils::primitives::to_string(q_js_rt, &res_ref)
    }
}

#[allow(dead_code)]
pub fn is_function(q_js_rt: &QuickJsRuntime, obj_ref: &JSValueRef) -> bool {
    if obj_ref.is_object() {
        let res = unsafe { q::JS_IsFunction(q_js_rt.context, *obj_ref.borrow_value()) };
        res != 0
    } else {
        false
    }
}

#[allow(dead_code)]
pub fn is_constructor(q_js_rt: &QuickJsRuntime, obj_ref: &JSValueRef) -> bool {
    if obj_ref.is_object() {
        let res = unsafe { q::JS_IsConstructor(q_js_rt.context, *obj_ref.borrow_value()) };
        res != 0
    } else {
        false
    }
}

pub fn call_constructor(
    q_js_rt: &QuickJsRuntime,
    constructor_ref: &JSValueRef,
    arguments: &[JSValueRef],
) -> Result<JSValueRef, EsError> {
    //extern "C" {
    //     pub fn JS_CallConstructor(
    //         ctx: *mut JSContext,
    //         func_obj: JSValue,
    //         argc: ::std::os::raw::c_int,
    //         argv: *mut JSValue,
    //     ) -> JSValue;
    // }

    let arg_count = arguments.len() as i32;

    let mut qargs = arguments
        .iter()
        .map(|arg| *arg.borrow_value())
        .collect::<Vec<_>>();

    let ret_val = unsafe {
        q::JS_CallConstructor(
            q_js_rt.context,
            *constructor_ref.borrow_value(),
            arg_count,
            qargs.as_mut_ptr(),
        )
    };
    Ok(JSValueRef::new(
        ret_val,
        false,
        true,
        "functions::call_constructor result",
    ))
}

#[allow(dead_code)]
pub fn new_native_function(
    q_js_rt: &QuickJsRuntime,
    name: &str,
    func: q::JSCFunction,
    arg_count: u32,
    is_constructor: bool,
) -> Result<JSValueRef, EsError> {
    let cname = make_cstring(name)?;
    let magic = 1;

    let cproto = if is_constructor {
        q::JSCFunctionEnum_JS_CFUNC_constructor
    } else {
        q::JSCFunctionEnum_JS_CFUNC_generic
    };

    let func_val = unsafe {
        q::JS_NewCFunction2(
            q_js_rt.context,
            func,
            cname.as_ptr(),
            arg_count as i32,
            cproto,
            magic,
        )
    };
    let func_ref = JSValueRef::new(
        func_val,
        false,
        true,
        "functions::new_native_function result",
    );

    if !func_ref.is_object() {
        Err(EsError::new_str("Could not create new_native_function"))
    } else {
        Ok(func_ref)
    }
}

#[allow(dead_code)]
pub fn new_native_function_data(
    q_js_rt: &QuickJsRuntime,
    func: q::JSCFunctionData,
    arg_count: u32,
    mut data: JSValueRef,
) -> Result<JSValueRef, EsError> {
    let magic = 1;
    let data_len = 1;

    let func_val = unsafe {
        q::JS_NewCFunctionData(
            q_js_rt.context,
            func,
            magic,
            arg_count as i32,
            data_len,
            data.borrow_value_mut(),
        )
    };
    let func_ref = JSValueRef::new(
        func_val,
        false,
        true,
        "functions::new_native_function_data result",
    );

    if !func_ref.is_object() {
        Err(EsError::new_str("Could not create new_native_function"))
    } else {
        Ok(func_ref)
    }
}

static CNAME: &str = "CallbackClass\0";

type Callback = dyn Fn(JSValueRef, Vec<JSValueRef>) -> Result<JSValueRef, EsError> + 'static;

thread_local! {
    static INSTANCE_ID_MAPPINGS: RefCell<HashMap<usize, Box<(usize, String)>>> = RefCell::new(HashMap::new());

    static CALLBACK_EXOTIC: RefCell<q::JSClassExoticMethods> = RefCell::new(q::JSClassExoticMethods {
        get_own_property: None,
        get_own_property_names: None,
        delete_property: None,
        define_own_property: None,
        has_property: None,
        get_property: None,
        set_property: None,
    });


    static CALLBACK_CLASS_DEF: RefCell<q::JSClassDef> = {
        CALLBACK_EXOTIC.with(|e_rc|{
            let exotic = &mut *e_rc.borrow_mut();
            RefCell::new(q::JSClassDef {
                class_name: CNAME.as_ptr() as *const c_char,
                finalizer: Some(callback_finalizer),
                gc_mark: None,
                call: None,
                exotic,
            })
        })
    };

    static CALLBACK_CLASS_ID: RefCell<u32> = {
        let mut c_id: u32 = 0;
        let class_id: u32 = unsafe { q::JS_NewClassID(&mut c_id) };
        log::trace!("got class id {}", class_id);

        CALLBACK_CLASS_DEF.with(|cd_rc| {
            let class_def = &*cd_rc.borrow();
            QuickJsRuntime::do_with(|q_js_rt| {
                let res = unsafe { q::JS_NewClass(q_js_rt.runtime, class_id, class_def) };
                log::trace!("callback: new class res {}", res);
                // todo res should be 0 for ok
            });
        });

        RefCell::new(class_id)
    };

    static CALLBACK_REGISTRY: RefCell<AutoIdMap<Box<Callback>>> = {
        RefCell::new(AutoIdMap::new_with_max_size(i32::MAX as usize))
    };

    static CALLBACK_IDS: RefCell<HashSet<Box<i32>>> = RefCell::new(HashSet::new());
}

#[allow(dead_code)]
pub fn new_function<F>(
    q_js_rt: &QuickJsRuntime,
    _name: &str,
    func: F,
    _arg_count: u32,
) -> Result<JSValueRef, EsError>
where
    F: Fn(JSValueRef, Vec<JSValueRef>) -> Result<JSValueRef, EsError> + 'static,
{
    // put func in map, retrieve on call.. delete on destroy
    // create a new class_def for callbacks, with a finalize
    // use setproto to bind class to function
    // use autoidmap to store callbacks and generate ID's
    // create function with newCFunctionData and put id in data

    let callback_id = CALLBACK_REGISTRY.with(|registry_rc| {
        let registry = &mut *registry_rc.borrow_mut();
        registry.insert(Box::new(func))
    });
    log::trace!("new_function callback_id = {}", callback_id);

    let data = primitives::from_i32(callback_id as i32);
    let func_ref = new_native_function_data(q_js_rt, Some(callback_function), 0, data)?;

    let callback_class_id = CALLBACK_CLASS_ID.with(|rc| *rc.borrow());

    let class_val: q::JSValue =
        unsafe { q::JS_NewObjectClass(q_js_rt.context, callback_class_id as i32) };

    let class_val_ref =
        JSValueRef::new(class_val, false, true, "functions::new_function class_val");

    if class_val_ref.is_exception() {
        return if let Some(e) = q_js_rt.get_exception() {
            Err(e)
        } else {
            Err(EsError::new_str("could not create callback class"))
        };
    }

    CALLBACK_IDS.with(|rc| {
        let ids = &mut *rc.borrow_mut();
        let mut bx = Box::new(callback_id as i32);

        let ibp: &mut i32 = &mut *bx;
        let info_ptr = ibp as *mut _ as *mut c_void;

        unsafe { q::JS_SetOpaque(*class_val_ref.borrow_value(), info_ptr) };

        ids.insert(bx);
    });

    objects::set_property2(q_js_rt, &func_ref, "_cb_fin_marker_", class_val_ref, 0)
        .ok()
        .expect("could not set cb marker");

    Ok(func_ref)
}

#[cfg(test)]
pub mod tests {
    use crate::esruntime::EsRuntime;
    use crate::esscript::EsScript;
    use crate::quickjs_utils::functions::{
        call_function, call_to_string, invoke_member_function, new_function,
    };
    use crate::quickjs_utils::{functions, objects, primitives};
    use std::sync::Arc;
    use std::time::Duration;

    #[test]
    pub fn test_invoke() {
        let rt: Arc<EsRuntime> = crate::esruntime::tests::TEST_ESRT.clone();
        let _io = rt.add_to_event_queue_sync(|q_js_rt| {
            let obj_ref = q_js_rt
                .eval(EsScript::new(
                    "test_to_invoke.es",
                    "({func: function(a, b) {return a*b}});",
                ))
                .ok()
                .expect("test_to_invoke.es failed");

            let res = invoke_member_function(
                q_js_rt,
                &obj_ref,
                "func",
                vec![primitives::from_i32(12), primitives::from_i32(14)],
            )
            .ok()
            .expect("func failed");

            q_js_rt.gc();

            assert!(res.is_i32());
            assert_eq!(primitives::to_i32(&res).ok().expect("wtf?"), (12 * 14));
        });
    }

    #[test]
    pub fn test_ret_refcount() {
        let rt: Arc<EsRuntime> = crate::esruntime::tests::TEST_ESRT.clone();
        let io = rt.add_to_event_queue_sync(|q_js_rt| {
            let func_ref = q_js_rt
                .eval(EsScript::new(
                    "test_ret_refcount.es",
                    "this.test = {q: {}}; let global = this; (function(a, b){global.test.a = a; return {a: 1};});",
                ))
                .ok()
                .expect("aa");
            assert_eq!(func_ref.get_ref_count(), 1);

            let a = objects::create_object(q_js_rt).ok().unwrap();
            let b = objects::create_object(q_js_rt).ok().unwrap();

            assert_eq!(1, a.get_ref_count());
            assert_eq!(1, b.get_ref_count());

            let i_res = call_function(q_js_rt, &func_ref, vec![a.clone(), b.clone()], None)
                .ok()
                .expect("a");
            assert!(i_res.is_object());
            assert_eq!(i_res.get_ref_count(), 1);

            assert_eq!(2, a.get_ref_count());
            assert_eq!(1, b.get_ref_count());

            let q_ref = q_js_rt.eval(EsScript::new("test_ret_refcount2.es", "test.q;")).ok().expect("get q failed");
            assert_eq!(2, q_ref.get_ref_count());
            let _ = call_function(q_js_rt, &func_ref, vec![primitives::from_i32(123), q_ref], None)
                .ok()
                .expect("b");
            let q_ref = q_js_rt.eval(EsScript::new("test_ret_refcount2.es", "test.q;")).ok().expect("get q failed");
            assert_eq!(2, q_ref.get_ref_count());
            let _ = call_function(q_js_rt, &func_ref, vec![q_ref, primitives::from_i32(123)], None)
                .ok()
                .expect("b");
            let q_ref = q_js_rt.eval(EsScript::new("test_ret_refcount2.es", "test.q;")).ok().expect("get q failed");
            assert_eq!(3, q_ref.get_ref_count());

            // cleanup
            q_js_rt.eval(EsScript::new("cleanup.es", "this.test = null;")).ok().unwrap();

            true
        });
        assert!(io);
    }

    #[test]
    pub fn test_to_string() {
        let rt: Arc<EsRuntime> = crate::esruntime::tests::TEST_ESRT.clone();
        let io = rt.add_to_event_queue_sync(|q_js_rt| {
            let i = primitives::from_i32(480);
            let i_s = call_to_string(q_js_rt, &i)
                .ok()
                .expect("to_string failed on i");
            assert_eq!(i_s.as_str(), "480");

            let b = primitives::from_bool(true);
            let b_s = call_to_string(q_js_rt, &b)
                .ok()
                .expect("to_string failed on b");
            assert_eq!(b_s.as_str(), "true");

            true
        });
        assert!(io);
    }

    #[test]
    pub fn test_call() {
        let rt: Arc<EsRuntime> = crate::esruntime::tests::TEST_ESRT.clone();
        let io = rt.add_to_event_queue_sync(|q_js_rt| {
            let func_ref = q_js_rt
                .eval(EsScript::new(
                    "test_call.es",
                    "(function(a, b){return ((a || 7)*(b || 7));});",
                ))
                .ok()
                .expect("could not get func obj");

            let res = call_function(
                q_js_rt,
                &func_ref,
                vec![primitives::from_i32(8), primitives::from_i32(6)],
                None,
            );
            if res.is_err() {
                panic!("test_call failed: {}");
            }
            let res_val = res.ok().unwrap();

            q_js_rt.gc();

            assert!(res_val.is_i32());

            assert_eq!(primitives::to_i32(&res_val).ok().unwrap(), 6 * 8);

            true
        });
        assert!(io)
    }

    #[test]
    fn test_callback() {
        let rt: Arc<EsRuntime> = crate::esruntime::tests::TEST_ESRT.clone();

        rt.eval_sync(EsScript::new("test_callback1.es", "let test_callback_563 = function(cb){console.log('before invoke cb');let result = cb(1, true, 'foobar');console.log('after invoke cb. got:' + result);};")).ok().expect("script failed");

        rt.add_to_event_queue_sync(|q_js_rt| {
            let mut cb_ref = new_function(
                q_js_rt,
                "cb",
                |_this_ref, _args| {
                    log::trace!("native callback invoked");
                    Ok(primitives::from_i32(983))
                },
                3,
            )
            .ok()
            .expect("could not create function");

            assert_eq!(1, cb_ref.get_ref_count());

            cb_ref.label("cb_ref at test_callback");

            let func_ref = q_js_rt
                .eval(EsScript::new("", "(test_callback_563);"))
                .ok()
                .expect("could not get function");

            assert_eq!(2, func_ref.get_ref_count());

            let res = call_function(q_js_rt, &func_ref, vec![cb_ref], None);
            if res.is_err() {
                let err = res.err().unwrap();
                log::error!("could not invoke test_callback_563: {}", err);
                panic!("could not invoke test_callback_563: {}", err);
            }
            res.ok().expect("could not invoke test_callback_563");
        });
        log::trace!("done with cb");
        rt.add_to_event_queue_sync(|q_js_rt| {
            q_js_rt.gc();
        });
        std::thread::sleep(Duration::from_secs(1));
    }

    #[test]
    fn test_callback_arg_ref_ct() {
        let rt: Arc<EsRuntime> = crate::esruntime::tests::TEST_ESRT.clone();

        rt.add_to_event_queue_sync(|q_js_rt| {

            let func_ref = q_js_rt.eval(EsScript::new(
                "test_callback3.es",
                "let test_callback_845 = function(cb){let obj = {}; cb(obj);cb(obj);cb(obj);}; test_callback_845;",
            ))
                .ok()
                .expect("script failed");

            let cb_ref = new_function(
                q_js_rt,
                "cb",
                |_this_ref, args| {
                    log::trace!("native callback invoked");
                    assert_eq!(args[0].get_ref_count(), 2);

                    Ok(primitives::from_i32(983))
                },
                3,
            )
            .ok()
            .expect("could not create function");
            functions::call_function(q_js_rt, &func_ref, vec![cb_ref], None).ok().expect("test_callback_845 failed");
        });
        log::trace!("done with cb");
        std::thread::sleep(Duration::from_secs(1));
        rt.add_to_event_queue_sync(|q_js_rt| {
            q_js_rt.gc();
        });
        std::thread::sleep(Duration::from_secs(1));
    }
}

unsafe extern "C" fn callback_finalizer(_rt: *mut q::JSRuntime, val: q::JSValue) {
    trace!("callback_finalizer called");

    let callback_class_id = CALLBACK_CLASS_ID.with(|rc| *rc.borrow());
    let info_ptr: *mut c_void = q::JS_GetOpaque(val, callback_class_id);
    let callback_id: i32 = *(info_ptr as *mut i32);

    trace!("callback_finalizer called, id={}", callback_id);

    CALLBACK_IDS.with(|rc| {
        let ids = &mut *rc.borrow_mut();
        ids.remove(&callback_id);
    });
    CALLBACK_REGISTRY.with(|rc| {
        let registry = &mut *rc.borrow_mut();

        let rid = callback_id as usize;
        trace!("callback_finalizer remove id={}", rid);
        let _ = registry.remove(&rid);
    });
}

unsafe extern "C" fn callback_function(
    _ctx: *mut q::JSContext,
    this_val: q::JSValue,
    argc: ::std::os::raw::c_int,
    argv: *mut q::JSValue,
    _magic: ::std::os::raw::c_int,
    func_data: *mut q::JSValue,
) -> q::JSValue {
    trace!("callback_function called");

    // todo run multiple times and check refcount not growing for data, this and args

    let data_ref = JSValueRef::new(*func_data, false, false, "callback_function func_data");
    let callback_id = primitives::to_i32(&data_ref)
        .ok()
        .expect("failed to get callback_id");

    trace!("callback_function id = {}", callback_id);

    CALLBACK_REGISTRY.with(|registry_rc| {
        let registry = &*registry_rc.borrow();
        if let Some(callback) = registry.get(&(callback_id as usize)) {
            QuickJsRuntime::do_with(|q_js_rt| {
                let arg_slice = std::slice::from_raw_parts(argv, argc as usize);
                let args_vec: Vec<JSValueRef> = arg_slice
                    .iter()
                    .map(|raw| JSValueRef::new(*raw, false, false, "callback_function arg"))
                    .collect::<Vec<_>>();

                let this_ref =
                    JSValueRef::new(this_val, false, false, "callback_function this_val");

                let callback_res: Result<JSValueRef, EsError> = callback(this_ref, args_vec);

                match callback_res {
                    Ok(res) => res.consume_value_no_decr_rc(),
                    Err(e) => {
                        let message =
                            format!("\n{} at\nnative_code\n{}", e.get_message(), e.get_stack());
                        let err = errors::new_error(
                            q_js_rt,
                            e.get_name(),
                            message.as_str(),
                            e.get_stack(),
                        )
                        .ok()
                        .expect("could not create err");
                        errors::throw(q_js_rt, err)
                    }
                }
            })
        } else {
            panic!("callback not found");
        }
    })
}
