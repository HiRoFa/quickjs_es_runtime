use crate::quickjsrealmadapter::QuickJsRealmAdapter;
use crate::quickjsvalueadapter::QuickJsValueAdapter;
use std::sync::Arc;

pub trait ScriptModuleLoader {
    fn normalize_path(
        &self,
        realm: &QuickJsRealmAdapter,
        ref_path: &str,
        path: &str,
    ) -> Option<String>;
    fn load_module(&self, realm: &QuickJsRealmAdapter, absolute_path: &str) -> String;
}

pub trait CompiledModuleLoader {
    fn normalize_path(
        &self,
        realm: &QuickJsRealmAdapter,
        ref_path: &str,
        path: &str,
    ) -> Option<String>;
    fn load_module(&self, realm: &QuickJsRealmAdapter, absolute_path: &str) -> Arc<Vec<u8>>;
}

pub trait NativeModuleLoader {
    fn has_module(&self, realm: &QuickJsRealmAdapter, module_name: &str) -> bool;
    fn get_module_export_names(&self, realm: &QuickJsRealmAdapter, module_name: &str) -> Vec<&str>;
    fn get_module_exports(
        &self,
        realm: &QuickJsRealmAdapter,
        module_name: &str,
    ) -> Vec<(&str, QuickJsValueAdapter)>;
}
