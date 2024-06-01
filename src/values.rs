use crate::facades::QuickjsRuntimeFacadeInner;
use crate::jsutils::{JsError, JsValueType};
use crate::quickjsrealmadapter::QuickJsRealmAdapter;
use crate::quickjsvalueadapter::QuickJsValueAdapter;
use crate::reflection::JsProxyInstanceId;
use futures::executor::block_on;
use futures::Future;
use hirofa_utils::debug_mutex::DebugMutex;
use serde::Serialize;
use serde_json::Value;
use std::collections::HashMap;
use std::error::Error;
use std::fmt::{Debug, Formatter};
use std::pin::Pin;
use std::sync::atomic::{AtomicU16, Ordering};
use std::sync::{Arc, Weak};
use std::time::Duration;
use string_cache::DefaultAtom;

pub struct CachedJsObjectRef {
    pub(crate) id: i32,
    rti: Weak<QuickjsRuntimeFacadeInner>,
    realm_id: String,
    drop_action: DebugMutex<Option<Box<dyn FnOnce() + Send>>>,
}

pub struct CachedJsPromiseRef {
    pub cached_object: CachedJsObjectRef,
}

pub struct CachedJsArrayRef {
    pub cached_object: CachedJsObjectRef,
}

pub struct CachedJsFunctionRef {
    pub cached_object: CachedJsObjectRef,
}

impl CachedJsObjectRef {
    pub(crate) fn new(realm: &QuickJsRealmAdapter, obj: QuickJsValueAdapter) -> Self {
        let id = realm.cache_object(obj);
        let rti_ref = realm.get_runtime_facade_inner();

        let drop_id = id;
        let drop_realm_name = realm.get_realm_id().to_string();

        Self::new2(
            id,
            rti_ref.clone(),
            realm.get_realm_id().to_string(),
            move || {
                if let Some(rti) = rti_ref.upgrade() {
                    rti.add_rt_task_to_event_loop_void(move |rt| {
                        if let Some(realm) = rt.get_realm(drop_realm_name.as_str()) {
                            realm.dispose_cached_object(drop_id);
                        }
                    })
                }
            },
        )
    }
    fn new2<F: FnOnce() + Send + 'static>(
        id: i32,
        rti: Weak<QuickjsRuntimeFacadeInner>,
        realm_name: String,
        drop_action: F,
    ) -> Self {
        Self {
            id,
            rti,
            realm_id: realm_name,
            drop_action: DebugMutex::new(
                Some(Box::new(drop_action)),
                "CachedJsObjectRef.drop_action",
            ),
        }
    }
    pub async fn to_json_string(&self) -> Result<String, JsError> {
        let id = self.id;
        let realm_name = self.realm_id.clone();
        let rti = self.rti.upgrade().expect("invalid state");
        rti.add_rt_task_to_event_loop(move |rt| {
            if let Some(realm) = rt.get_realm(realm_name.as_str()) {
                //let realm: JsRealmAdapter<JsRuntimeAdapterType = (), JsValueAdapterType = ()> = realm;
                realm.with_cached_object(id, |obj| realm.json_stringify(obj, None))
            } else {
                Err(JsError::new_str("no such realm"))
            }
        })
        .await
    }
    pub fn get_object_sync(&self) -> Result<HashMap<String, JsValueFacade>, JsError> {
        block_on(self.get_object())
    }

    pub async fn get_object(&self) -> Result<HashMap<String, JsValueFacade>, JsError> {
        let id = self.id;
        let realm_name = self.realm_id.clone();
        let rti = self.rti.upgrade().expect("invalid state");
        rti.add_rt_task_to_event_loop(move |rt| {
            if let Some(realm) = rt.get_realm(realm_name.as_str()) {
                //let realm: JsRealmAdapter = realm;
                let mut ret = HashMap::new();
                let results = realm.with_cached_object(id, |obj| {
                    realm.traverse_object(obj, |name, value| {
                        //
                        Ok((name.to_string(), realm.to_js_value_facade(value)))
                    })
                })?;
                for result in results {
                    ret.insert(result.0, result.1?);
                }
                Ok(ret)
            } else {
                Err(JsError::new_str("no such realm"))
            }
        })
        .await
    }
    pub async fn get_serde_value(&self) -> Result<serde_json::Value, JsError> {
        let id = self.id;
        let realm_name = self.realm_id.clone();
        let rti = self.rti.upgrade().expect("invalid state");
        rti.add_rt_task_to_event_loop(move |rt| {
            if let Some(realm) = rt.get_realm(realm_name.as_str()) {
                realm.with_cached_object(id, |obj| realm.value_adapter_to_serde_value(obj))
            } else {
                Err(JsError::new_str("no such realm"))
            }
        })
        .await
    }
    pub fn with_obj_sync<
        S: Send + 'static,
        C: FnOnce(&QuickJsRealmAdapter, &QuickJsValueAdapter) -> S + Send + 'static,
    >(
        &self,
        consumer: C,
    ) -> Result<S, JsError> {
        let id = self.id;
        let realm_id = self.realm_id.clone();
        let rti = self.rti.upgrade().expect("invalid state");
        rti.exe_rt_task_in_event_loop(move |rt| {
            if let Some(realm) = rt.get_realm(realm_id.as_str()) {
                Ok(realm.with_cached_object(id, |obj| consumer(realm, obj)))
            } else {
                Err(JsError::new_str("Realm was disposed"))
            }
        })
    }
    pub fn with_obj_void<
        S: Send + 'static,
        C: FnOnce(&QuickJsRealmAdapter, &QuickJsValueAdapter) -> S + Send + 'static,
    >(
        &self,
        consumer: C,
    ) {
        let id = self.id;
        let realm_id = self.realm_id.clone();
        let rti = self.rti.upgrade().expect("invalid state");
        rti.add_rt_task_to_event_loop_void(move |rt| {
            if let Some(realm) = rt.get_realm(realm_id.as_str()) {
                realm.with_cached_object(id, |obj| consumer(realm, obj));
            } else {
                log::error!("no such realm");
            }
        })
    }
    pub async fn with_obj<
        S: Send + 'static,
        C: FnOnce(&QuickJsRealmAdapter, &QuickJsValueAdapter) -> S + Send + 'static,
    >(
        &self,
        consumer: C,
    ) -> Result<S, JsError> {
        let id = self.id;
        let realm_id = self.realm_id.clone();
        let rti = self.rti.upgrade().expect("invalid state");
        rti.add_rt_task_to_event_loop(move |rt| {
            if let Some(realm) = rt.get_realm(realm_id.as_str()) {
                Ok(realm.with_cached_object(id, |obj| consumer(realm, obj)))
            } else {
                Err(JsError::new_str("Realm was disposed"))
            }
        })
        .await
    }
}

impl Drop for CachedJsObjectRef {
    fn drop(&mut self) {
        let lck = &mut *self.drop_action.lock("drop").unwrap();
        if let Some(da) = lck.take() {
            da();
        }
    }
}

impl CachedJsPromiseRef {
    pub async fn get_serde_value(&self) -> Result<serde_json::Value, JsError> {
        self.cached_object.get_serde_value().await
    }
    pub async fn to_json_string(&self) -> Result<String, JsError> {
        self.cached_object.to_json_string().await
    }

    pub fn get_promise_result_sync(&self) -> Result<Result<JsValueFacade, JsValueFacade>, JsError> {
        let rx = self.get_promise_result_receiver();
        rx.recv()
            .map_err(|e| JsError::new_string(format!("get_promise_result_sync/1: {e}")))?
    }

    pub fn get_promise_result_sync_timeout(
        &self,
        timeout: Option<Duration>,
    ) -> Result<Result<JsValueFacade, JsValueFacade>, JsError> {
        let rx = self.get_promise_result_receiver();
        let res = if let Some(timeout) = timeout {
            rx.recv_timeout(timeout)
                .map_err(|e| JsError::new_string(format!("get_promise_result_sync_timeout/1: {e}")))
        } else {
            rx.recv()
                .map_err(|e| JsError::new_string(format!("get_promise_result_sync_timeout/2: {e}")))
        };
        res?
    }

    pub async fn get_promise_result(
        &self,
    ) -> Result<Result<JsValueFacade, JsValueFacade>, JsError> {
        let rx = self.get_promise_result_receiver();
        rx.into_recv_async()
            .await
            .map_err(|e| JsError::new_string(format!("{e}")))?
    }

    pub fn get_promise_result_receiver(
        &self,
    ) -> flume::Receiver<Result<Result<JsValueFacade, JsValueFacade>, JsError>> {
        let (tx, rx) = flume::bounded(1);

        let tx1 = tx.clone();
        let tx2 = tx.clone();

        let state = Arc::new(AtomicU16::new(0));
        let state_then = state.clone();
        let state_catch = state.clone();

        self.cached_object.with_obj_void(move |realm, obj| {
            let res = || {
                let then_func = realm.create_function(
                    "then",
                    move |realm, _this, args| {
                        //

                        state_then.fetch_add(1, Ordering::Relaxed);

                        let resolution = &args[0];
                        let send_res = match realm.to_js_value_facade(resolution) {
                            Ok(vf) => tx1.send(Ok(Ok(vf))),
                            Err(conv_err) => tx1.send(Err(conv_err)),
                        };

                        send_res.map_err(|e| {
                            JsError::new_string(format!(
                                "could not send: {e} state:{}",
                                state_then.load(Ordering::Relaxed)
                            ))
                        })?;
                        realm.create_undefined()
                    },
                    1,
                )?;
                let catch_func = realm.create_function(
                    "catch",
                    move |realm, _this, args| {
                        //

                        state_catch.fetch_add(16, Ordering::Relaxed);

                        let rejection = &args[0];
                        let send_res = match realm.to_js_value_facade(rejection) {
                            Ok(vf) => tx2.send(Ok(Err(vf))),
                            Err(conv_err) => tx2.send(Err(conv_err)),
                        };

                        send_res.map_err(|e| {
                            JsError::new_string(format!(
                                "could not send: {e} state:{}",
                                state_catch.load(Ordering::Relaxed)
                            ))
                        })?;
                        realm.create_undefined()
                    },
                    1,
                )?;

                realm.add_promise_reactions(obj, Some(then_func), Some(catch_func), None)?;
                Ok(())
            };
            match res() {
                Ok(_) => {}
                Err(e) => {
                    state.fetch_add(64, Ordering::Relaxed);
                    log::error!("failed to add promise reactions {}", e);
                    match tx.send(Err(e)) {
                        Ok(_) => {}
                        Err(e) => {
                            log::error!("failed to resolve 47643: {}", e);
                        }
                    }
                }
            }
        });

        rx
    }
}

impl CachedJsArrayRef {
    pub async fn get_serde_value(&self) -> Result<serde_json::Value, JsError> {
        self.cached_object.get_serde_value().await
    }
    pub async fn to_json_string(&self) -> Result<String, JsError> {
        self.cached_object.to_json_string().await
    }
    pub async fn get_array(&self) -> Result<Vec<JsValueFacade>, JsError> {
        self.cached_object
            .with_obj(|realm, arr| {
                let mut vec: Vec<JsValueFacade> = vec![];
                realm.traverse_array_mut(arr, |_index, element| {
                    vec.push(realm.to_js_value_facade(element)?);
                    Ok(())
                })?;
                Ok(vec)
            })
            .await?
    }
}

impl CachedJsFunctionRef {
    pub async fn get_serde_value(&self) -> Result<serde_json::Value, JsError> {
        self.cached_object.get_serde_value().await
    }

    pub fn invoke_function(
        &self,
        args: Vec<JsValueFacade>,
    ) -> impl Future<Output = Result<JsValueFacade, JsError>> + Send {
        //Pin<Box<dyn futures::Future<Output = Result<JsValueFacade, JsError>>>>
        let cached_obj_id = self.cached_object.id;
        let realm_id = self.cached_object.realm_id.clone();
        let rti = self.cached_object.rti.upgrade().expect("invalid state");
        rti.add_rt_task_to_event_loop(move |rt| {
            //
            if let Some(realm) = rt.get_realm(realm_id.as_str()) {
                realm.with_cached_object(cached_obj_id, move |func_adapter| {
                    let mut adapter_args = vec![];
                    for arg in args {
                        adapter_args.push(realm.from_js_value_facade(arg)?);
                    }

                    let adapter_refs: Vec<&QuickJsValueAdapter> = adapter_args.iter().collect();

                    let val_adapter = realm.invoke_function(None, func_adapter, &adapter_refs)?;

                    realm.to_js_value_facade(&val_adapter)
                })
            } else {
                Ok(JsValueFacade::Null)
            }
        })
    }
    pub fn invoke_function_sync(&self, args: Vec<JsValueFacade>) -> Result<JsValueFacade, JsError> {
        self.cached_object.with_obj_sync(|realm, func_adapter| {
            //
            let mut adapter_args = vec![];
            for arg in args {
                adapter_args.push(realm.from_js_value_facade(arg)?);
            }

            let adapter_refs: Vec<&QuickJsValueAdapter> = adapter_args.iter().collect();

            let val_adapter = realm.invoke_function(None, func_adapter, &adapter_refs)?;

            realm.to_js_value_facade(&val_adapter)
        })?
    }
}

pub enum TypedArrayType {
    Uint8,
}

/// The JsValueFacade is a Send-able representation of a value in the Script engine
#[allow(clippy::type_complexity)]
pub enum JsValueFacade {
    I32 {
        val: i32,
    },
    F64 {
        val: f64,
    },
    String {
        val: DefaultAtom,
    },
    Boolean {
        val: bool,
    },
    JsObject {
        // obj which is a ref to obj in Js
        cached_object: CachedJsObjectRef,
    },
    JsPromise {
        cached_promise: CachedJsPromiseRef,
    },
    JsArray {
        cached_array: CachedJsArrayRef,
    },
    JsFunction {
        cached_function: CachedJsFunctionRef,
    },
    // obj created from rust
    Object {
        val: HashMap<String, JsValueFacade>,
    },
    // array created from rust
    Array {
        val: Vec<JsValueFacade>,
    },
    // promise created from rust which will run an async producer
    Promise {
        producer: DebugMutex<
            Option<Pin<Box<dyn Future<Output = Result<JsValueFacade, JsError>> + Send + 'static>>>,
        >,
    },
    // Function created from rust
    Function {
        name: String,
        arg_count: u32,
        func: Arc<Box<dyn Fn(&[JsValueFacade]) -> Result<JsValueFacade, JsError> + Send + Sync>>,
    },
    JsError {
        val: JsError,
    },
    ProxyInstance {
        namespace: &'static [&'static str],
        class_name: &'static str,
        instance_id: JsProxyInstanceId,
    },
    TypedArray {
        buffer: Vec<u8>,
        array_type: TypedArrayType,
    },
    JsonStr {
        json: String,
    },
    SerdeValue {
        value: serde_json::Value,
    },
    Null,
    Undefined,
}

impl JsValueFacade {
    pub fn from_serializable<T: Serialize>(obj: &T) -> Result<Self, Box<dyn Error>> {
        let json = serde_json::to_string(obj)?;
        Ok(Self::JsonStr { json })
    }

    pub fn new_i32(val: i32) -> Self {
        Self::I32 { val }
    }
    pub fn new_f64(val: f64) -> Self {
        Self::F64 { val }
    }
    pub fn new_bool(val: bool) -> Self {
        Self::Boolean { val }
    }
    pub fn new_str_atom(val: DefaultAtom) -> Self {
        Self::String { val }
    }
    pub fn new_str(val: &str) -> Self {
        Self::String {
            val: DefaultAtom::from(val),
        }
    }
    pub fn new_string(val: String) -> Self {
        Self::String {
            val: DefaultAtom::from(val),
        }
    }
    pub fn new_callback<
        F: Fn(&[JsValueFacade]) -> Result<JsValueFacade, JsError> + Send + Sync + 'static,
    >(
        callback: F,
    ) -> Self {
        Self::Function {
            name: "".to_string(),
            arg_count: 0,
            func: Arc::new(Box::new(callback)),
        }
    }
    pub fn new_function<
        F: Fn(&[JsValueFacade]) -> Result<JsValueFacade, JsError> + Send + Sync + 'static,
    >(
        name: &str,
        function: F,
        arg_count: u32,
    ) -> Self {
        Self::Function {
            name: name.to_string(),
            arg_count,
            func: Arc::new(Box::new(function)),
        }
    }
    /// create a new promise with a producer which will run async in a threadpool
    pub fn new_promise<R, P, M>(producer: P) -> Self
    where
        P: Future<Output = Result<JsValueFacade, JsError>> + Send + 'static,
    {
        JsValueFacade::Promise {
            producer: DebugMutex::new(Some(Box::pin(producer)), "JsValueFacade::Promise.producer"),
        }
    }

    pub fn is_i32(&self) -> bool {
        matches!(self, JsValueFacade::I32 { .. })
    }
    pub fn is_f64(&self) -> bool {
        matches!(self, JsValueFacade::F64 { .. })
    }
    pub fn is_bool(&self) -> bool {
        matches!(self, JsValueFacade::Boolean { .. })
    }
    pub fn is_string(&self) -> bool {
        matches!(self, JsValueFacade::String { .. })
    }
    pub fn is_js_promise(&self) -> bool {
        matches!(self, JsValueFacade::JsPromise { .. })
    }
    pub fn is_js_object(&self) -> bool {
        matches!(self, JsValueFacade::JsObject { .. })
    }
    pub fn is_js_array(&self) -> bool {
        matches!(self, JsValueFacade::JsArray { .. })
    }

    pub fn get_i32(&self) -> i32 {
        match self {
            JsValueFacade::I32 { val } => *val,
            _ => {
                panic!("Not an i32");
            }
        }
    }
    pub fn get_f64(&self) -> f64 {
        match self {
            JsValueFacade::F64 { val } => *val,
            _ => {
                panic!("Not an f64");
            }
        }
    }
    pub fn get_bool(&self) -> bool {
        match self {
            JsValueFacade::Boolean { val } => *val,
            _ => {
                panic!("Not a boolean");
            }
        }
    }
    pub fn get_str(&self) -> &str {
        match self {
            JsValueFacade::String { val } => val,
            _ => {
                panic!("Not a string");
            }
        }
    }
    pub fn get_str_atom(&self) -> &DefaultAtom {
        match self {
            JsValueFacade::String { val } => val,
            _ => {
                panic!("Not a string");
            }
        }
    }
    pub fn is_null_or_undefined(&self) -> bool {
        matches!(self, JsValueFacade::Null | JsValueFacade::Undefined)
    }
    pub fn get_value_type(&self) -> JsValueType {
        match self {
            JsValueFacade::I32 { .. } => JsValueType::I32,
            JsValueFacade::F64 { .. } => JsValueType::F64,
            JsValueFacade::String { .. } => JsValueType::String,
            JsValueFacade::Boolean { .. } => JsValueType::Boolean,
            JsValueFacade::JsObject { .. } => JsValueType::Object,
            JsValueFacade::Null => JsValueType::Null,
            JsValueFacade::Undefined => JsValueType::Undefined,
            JsValueFacade::Object { .. } => JsValueType::Object,
            JsValueFacade::Array { .. } => JsValueType::Array,
            JsValueFacade::Promise { .. } => JsValueType::Promise,
            JsValueFacade::Function { .. } => JsValueType::Function,
            JsValueFacade::JsPromise { .. } => JsValueType::Promise,
            JsValueFacade::JsArray { .. } => JsValueType::Array,
            JsValueFacade::JsFunction { .. } => JsValueType::Function,
            JsValueFacade::JsError { .. } => JsValueType::Error,
            JsValueFacade::ProxyInstance { .. } => JsValueType::Object,
            JsValueFacade::TypedArray { .. } => JsValueType::Object,
            JsValueFacade::JsonStr { .. } => JsValueType::Object,
            JsValueFacade::SerdeValue { value } => match value {
                serde_json::Value::Null => JsValueType::Null,
                serde_json::Value::Bool(_) => JsValueType::Boolean,
                serde_json::Value::Number(_) => {
                    if value.is_i64() {
                        let num = value.as_i64().unwrap();
                        if num <= i32::MAX as i64 {
                            JsValueType::I32
                        } else {
                            JsValueType::F64
                        }
                    } else if value.is_f64() {
                        JsValueType::F64
                    } else {
                        // u64
                        let num = value.as_u64().unwrap();
                        if num <= i32::MAX as u64 {
                            JsValueType::I32
                        } else {
                            JsValueType::F64
                        }
                    }
                }
                serde_json::Value::String(_) => JsValueType::String,
                serde_json::Value::Array(_) => JsValueType::Array,
                serde_json::Value::Object(_) => JsValueType::Object,
            },
        }
    }
    pub fn stringify(&self) -> String {
        match self {
            JsValueFacade::I32 { val } => {
                format!("I32: {val}")
            }
            JsValueFacade::F64 { val } => {
                format!("F64: {val}")
            }
            JsValueFacade::String { val } => {
                format!("String: {val}")
            }
            JsValueFacade::Boolean { val } => {
                format!("Boolean: {val}")
            }
            JsValueFacade::JsObject { cached_object } => {
                format!(
                    "JsObject: [{}.{}]",
                    cached_object.realm_id, cached_object.id
                )
            }
            JsValueFacade::JsPromise { cached_promise } => {
                format!(
                    "JsPromise: [{}.{}]",
                    cached_promise.cached_object.realm_id, cached_promise.cached_object.id
                )
            }
            JsValueFacade::JsArray { cached_array } => {
                format!(
                    "JsArray: [{}.{}]",
                    cached_array.cached_object.realm_id, cached_array.cached_object.id
                )
            }
            JsValueFacade::JsFunction { cached_function } => {
                format!(
                    "JsFunction: [{}.{}]",
                    cached_function.cached_object.realm_id, cached_function.cached_object.id
                )
            }
            JsValueFacade::Object { val } => {
                format!("Object: [len={}]", val.keys().len())
            }
            JsValueFacade::Array { val } => {
                format!("Array: [len={}]", val.len())
            }
            JsValueFacade::Promise { .. } => "Promise".to_string(),
            JsValueFacade::Function { .. } => "Function".to_string(),
            JsValueFacade::Null => "Null".to_string(),
            JsValueFacade::Undefined => "Undefined".to_string(),
            JsValueFacade::JsError { val } => format!("{val}"),
            JsValueFacade::ProxyInstance { .. } => "ProxyInstance".to_string(),
            JsValueFacade::TypedArray { .. } => "TypedArray".to_string(),
            JsValueFacade::JsonStr { json } => format!("JsonStr: '{json}'"),
            JsValueFacade::SerdeValue { value } => format!("Serde value: {value}"),
        }
    }
    pub async fn to_serde_value(&self) -> Result<serde_json::Value, JsError> {
        match self {
            JsValueFacade::I32 { val } => Ok(serde_json::Value::from(*val)),
            JsValueFacade::F64 { val } => Ok(serde_json::Value::from(*val)),
            JsValueFacade::String { val } => Ok(serde_json::Value::from(val.to_string())),
            JsValueFacade::Boolean { val } => Ok(serde_json::Value::from(*val)),
            JsValueFacade::JsObject { cached_object } => cached_object.get_serde_value().await,
            JsValueFacade::JsPromise { cached_promise } => cached_promise.get_serde_value().await,
            JsValueFacade::JsArray { cached_array } => cached_array.get_serde_value().await,
            JsValueFacade::JsFunction { .. } => Ok(Value::Null),
            JsValueFacade::Object { .. } => Ok(Value::Null),
            JsValueFacade::Array { .. } => Ok(Value::Null),
            JsValueFacade::Promise { .. } => Ok(Value::Null),
            JsValueFacade::Function { .. } => Ok(Value::Null),
            JsValueFacade::Null => Ok(Value::Null),
            JsValueFacade::Undefined => Ok(Value::Null),
            JsValueFacade::JsError { .. } => Ok(Value::Null),
            JsValueFacade::ProxyInstance { .. } => Ok(Value::Null),
            JsValueFacade::TypedArray { .. } => Ok(Value::Null),
            JsValueFacade::JsonStr { json } => Ok(serde_json::from_str(json).unwrap()),
            JsValueFacade::SerdeValue { value } => Ok(value.clone()),
        }
    }
    pub async fn to_json_string(&self) -> Result<String, JsError> {
        match self {
            JsValueFacade::I32 { val } => Ok(format!("{val}")),
            JsValueFacade::F64 { val } => Ok(format!("{val}")),
            JsValueFacade::String { val } => Ok(format!("'{}'", val.replace('\'', "\\'"))),
            JsValueFacade::Boolean { val } => Ok(format!("{val}")),
            JsValueFacade::JsObject { cached_object } => cached_object.to_json_string().await,
            JsValueFacade::JsPromise { cached_promise } => cached_promise.to_json_string().await,
            JsValueFacade::JsArray { cached_array } => cached_array.to_json_string().await,
            JsValueFacade::JsFunction { .. } => Ok("function () {}".to_string()),
            JsValueFacade::Object { .. } => Ok("{}".to_string()),
            JsValueFacade::Array { .. } => Ok("{}".to_string()),
            JsValueFacade::Promise { .. } => Ok("{}".to_string()),
            JsValueFacade::Function { .. } => Ok("function () {}".to_string()),
            JsValueFacade::Null => Ok("null".to_string()),
            JsValueFacade::Undefined => Ok("undefined".to_string()),
            JsValueFacade::JsError { val } => Ok(format!("'{val}'")),
            JsValueFacade::ProxyInstance { .. } => Ok("{}".to_string()),
            JsValueFacade::TypedArray { .. } => Ok("[]".to_string()),
            JsValueFacade::JsonStr { json } => Ok(json.clone()),
            JsValueFacade::SerdeValue { value } => Ok(serde_json::to_string(value).unwrap()),
        }
    }
}

impl Debug for JsValueFacade {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.stringify().as_str())
    }
}

pub trait JsValueConvertable {
    fn to_js_value_facade(self) -> JsValueFacade;
}

impl JsValueConvertable for serde_json::Value {
    fn to_js_value_facade(self) -> JsValueFacade {
        JsValueFacade::SerdeValue { value: self }
    }
}

impl JsValueConvertable for bool {
    fn to_js_value_facade(self) -> JsValueFacade {
        JsValueFacade::new_bool(self)
    }
}

impl JsValueConvertable for i32 {
    fn to_js_value_facade(self) -> JsValueFacade {
        JsValueFacade::new_i32(self)
    }
}

impl JsValueConvertable for f64 {
    fn to_js_value_facade(self) -> JsValueFacade {
        JsValueFacade::new_f64(self)
    }
}

impl JsValueConvertable for &str {
    fn to_js_value_facade(self) -> JsValueFacade {
        JsValueFacade::new_str(self)
    }
}

impl JsValueConvertable for String {
    fn to_js_value_facade(self) -> JsValueFacade {
        JsValueFacade::new_string(self)
    }
}

impl JsValueConvertable for Vec<u8> {
    fn to_js_value_facade(self) -> JsValueFacade {
        JsValueFacade::TypedArray {
            buffer: self,
            array_type: TypedArrayType::Uint8,
        }
    }
}

impl JsValueConvertable for Vec<JsValueFacade> {
    fn to_js_value_facade(self) -> JsValueFacade {
        JsValueFacade::Array { val: self }
    }
}

impl JsValueConvertable for HashMap<String, JsValueFacade> {
    fn to_js_value_facade(self) -> JsValueFacade {
        JsValueFacade::Object { val: self }
    }
}
/* todo
impl JsValueConvertable for Fn(&[JsValueFacade]) -> Result<JsValueFacade, JsError> + Send + Sync {
    fn to_js_value_facade(self) -> JsValueFacade {
        JsValueFacade::Function {
            name: "".to_string(),
            arg_count: 0,
            func: Arc::new(Box::new(self)),
        }
    }
}
 */
