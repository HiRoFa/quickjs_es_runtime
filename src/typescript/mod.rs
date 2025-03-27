// public transpile function which can also be used by gcs to transpile clientside ts

use crate::jsutils::JsError;
use crate::jsutils::Script;
use crate::quickjs_utils::modules::detect_module;
use crate::quickjsruntimeadapter::QuickJsRuntimeAdapter;
use std::cell::RefCell;
use std::collections::HashMap;
use std::io;
use std::str::FromStr;
use std::sync::Arc;
use swc::Compiler;
use swc_common::errors::{ColorConfig, Handler};
use swc_common::{FileName, SourceMap};

pub enum TargetVersion {
    Es3,
    Es5,
    Es2016,
    Es2020,
    Es2021,
    Es2022,
}

impl TargetVersion {
    fn as_str(&self) -> &str {
        match self {
            TargetVersion::Es3 => "es3",
            TargetVersion::Es5 => "es5",
            TargetVersion::Es2016 => "es2016",
            TargetVersion::Es2020 => "es2020",
            TargetVersion::Es2021 => "es2021",
            TargetVersion::Es2022 => "es2022",
        }
    }
}

pub struct TypeScriptTranspiler {
    minify: bool,
    mangle: bool,
    external_helpers: bool,
    target: TargetVersion,
    compiler: Compiler,
    source_map: Arc<SourceMap>,
}

impl TypeScriptTranspiler {
    pub fn new(target: TargetVersion, minify: bool, external_helpers: bool, mangle: bool) -> Self {
        let source_map = Arc::<SourceMap>::default();
        let compiler = swc::Compiler::new(source_map.clone());

        Self {
            minify,
            mangle,
            external_helpers,
            target,
            source_map,
            compiler,
        }
    }
    // todo custom target
    pub fn transpile(
        &self,
        code: &str,
        file_name: &str,
        is_module: bool,
    ) -> Result<(String, Option<String>), JsError> {
        let globals = swc_common::Globals::new();
        swc_common::GLOBALS.set(&globals, || {
            let handler = Handler::with_tty_emitter(
                ColorConfig::Auto,
                true,
                false,
                Some(self.source_map.clone()),
            );

            let fm = self
                .source_map
                .new_source_file(Arc::new(FileName::Custom(file_name.into())), code.into());

            let mangle_config = if self.mangle {
                r#"
                    {
                        "topLevel": false,
                        "keepClassNames": true
                    }
                "#
            } else {
                "false"
            };

            let minify_options = if self.minify {
                format!(
                    r#"
                "minify": {{
                  "compress": {{
                    "unused": true
                  }},
                  "format": {{
                    "comments": false
                  }},
                  "mangle": {mangle_config}
                }},
            "#
                )
            } else {
                r#"
                "minify": {
                  "format": {
                    "comments": false
                  }
                },
                "#
                .to_string()
            };

            let module = if is_module {
                r#"
                "module": {
                    "type": "es6",
                    "strict": true,
                    "strictMode": true,
                    "lazy": false,
                    "noInterop": false,
                    "ignoreDynamic": true
                },
                "#
            } else {
                ""
            };

            let cfg_json = format!(
                r#"

            {{
              "minify": {},
              "sourceMaps": true,
              {}
              "jsc": {{
                {}
                "externalHelpers": {},
                "parser": {{
                  "syntax": "typescript",
                  "jsx": true,
                  "tsx": true,
                  "decorators": true,
                  "decoratorsBeforeExport": true,
                  "dynamicImport": true,
                  "preserveAllComments": false
                }},
                "transform": {{
                  "legacyDecorator": true,
                  "decoratorMetadata": true,
                  "react": {{
                      "runtime": "classic",
                      "useBuiltins": true,
                      "refresh": true
                  }}
                }},
                "target": "{}",
                "keepClassNames": true
              }}
            }}

        "#,
                self.minify,
                module,
                minify_options,
                self.external_helpers,
                self.target.as_str()
            );

            log::trace!("using config {}", cfg_json);

            let cfg = serde_json::from_str(cfg_json.as_str())
                .map_err(|e| JsError::new_string(format!("{e}")))?;

            let ops = swc::config::Options {
                config: cfg,
                ..Default::default()
            };

            // todo see https://github.com/swc-project/swc/discussions/4126
            // for better example

            let res = self.compiler.process_js_file(fm, &handler, &ops);

            match res {
                Ok(to) => Ok((to.code, to.map)),
                Err(e) => Err(JsError::new_string(format!("transpile failed: {e}"))),
            }
        })
    }

    pub fn transpile_script(&self, script: &mut Script) -> Result<(), JsError> {
        if script.get_path().ends_with(".ts") {
            let code = script.get_code();

            let is_module = detect_module(code);

            let js = self.transpile(code, script.get_path(), is_module)?;
            log::debug!("map: {:?}", js.1);
            script.set_transpiled_code(js.0, js.1);
        }
        log::debug!(
            "TypeScriptPreProcessor:process file={} result = {}",
            script.get_path(),
            script.get_runnable_code()
        );

        Ok(())
    }
}

impl Default for TypeScriptTranspiler {
    fn default() -> Self {
        Self::new(TargetVersion::Es2020, false, true, false)
    }
}

thread_local! {
    // we store this in a thread local inb the worker thread so they are dropped when the runtimefacade is dropped
    static SOURCE_MAPS: RefCell<HashMap<String, String>> = RefCell::new(HashMap::new());
    static TRANSPILER: RefCell<TypeScriptTranspiler> = RefCell::new(TypeScriptTranspiler::new(TargetVersion::Es2020, false, true, false));
}

// fix stacktrace method
pub(crate) fn transpile_serverside(
    _rt: &QuickJsRuntimeAdapter,
    script: &mut Script,
) -> Result<(), JsError> {
    // transpile and store map in qjsrt

    // transpile
    TRANSPILER.with(|rc| {
        let transpiler: &TypeScriptTranspiler = &rc.borrow();
        transpiler.transpile_script(script)
    })?;

    // register in source_maps so fix_stack can use it later
    if let Some(map_str) = script.get_map() {
        SOURCE_MAPS.with(|rc| {
            let maps = &mut *rc.borrow_mut();
            maps.insert(script.get_path().to_string(), map_str.to_string());
        })
    }
    Ok(())
}

#[derive(Debug)]
struct StackEntry {
    function_name: String,
    file_name: String,
    line_number: Option<u32>,
    column_number: Option<u32>,
}

impl FromStr for StackEntry {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        // remove 'at '
        let s = &s[3..];

        let mut parts = s.splitn(2, ' ');
        let function_name = parts.next().unwrap_or("unnamed").to_string();
        let mut file_name = parts.next().unwrap_or("(unknown)").to_string();
        if file_name.starts_with('(') {
            file_name = file_name.as_str()[1..].to_string();
        }
        if file_name.ends_with(')') {
            file_name = file_name.as_str()[..file_name.len() - 1].to_string();
        }
        file_name = file_name.replace("://", "_double_point_placeholder_//");

        let parts: Vec<&str> = file_name.split(':').collect();

        let file_name = parts[0]
            .to_string()
            .replace("_double_point_placeholder_//", "://");
        let line_number = parts.get(1).and_then(|s| s.parse::<u32>().ok());
        let column_number = parts.get(2).and_then(|s| s.parse::<u32>().ok());

        Ok(StackEntry {
            function_name,
            file_name,
            column_number,
            line_number,
        })
    }
}

fn parse_stack_trace(stack_trace: &str) -> Result<Vec<StackEntry>, String> {
    let entries: Vec<StackEntry> = stack_trace
        .lines()
        .map(|line| line.trim())
        .filter(|line| !line.is_empty())
        .map(|line| line.parse::<StackEntry>())
        .collect::<Result<Vec<_>, _>>()?;

    Ok(entries)
}

fn serialize_stack(entries: &[StackEntry]) -> String {
    let mut result = String::new();

    for entry in entries {
        let fname_lnum = if let Some(line_number) = entry.line_number {
            if let Some(column_number) = entry.column_number {
                format!("{}:{line_number}:{column_number}", entry.file_name)
            } else {
                format!("{}:{line_number}", entry.file_name)
            }
        } else {
            entry.file_name.clone()
        };

        result.push_str(&format!("    at {} ({fname_lnum})", entry.function_name));

        result.push('\n');
    }

    result
}

pub(crate) fn unmap_stack_trace(stack_trace: &str) -> String {
    // todo: not the fastest way to impl this.. should I keep instances of source map instead of string? what does that do to mem consumtion?
    SOURCE_MAPS.with(|rc| fix_stack_trace(stack_trace, &rc.borrow()))
}

pub fn fix_stack_trace(stack_trace: &str, maps: &HashMap<String, String>) -> String {
    log::trace!("fix_stack_trace:\n{stack_trace}");

    match parse_stack_trace(stack_trace) {
        Ok(mut parsed_stack) => {
            for stack_trace_entry in parsed_stack.iter_mut() {
                if let Some(map_str) = maps.get(stack_trace_entry.file_name.as_str()) {
                    log::trace!(
                        "fix_stack_trace:found map for file {}:\n{map_str}",
                        stack_trace_entry.file_name.as_str()
                    );
                    if let Some(line_number) = stack_trace_entry.line_number {
                        log::trace!("lookup line number:{line_number}");
                        match swc::sourcemap::SourceMap::from_reader(io::Cursor::new(map_str)) {
                            Ok(source_map) => {
                                if let Some(original_location) = source_map.lookup_token(
                                    line_number-1,
                                    stack_trace_entry.column_number.unwrap_or(1)-1,
                                ) {
                                    let original_line = original_location.get_src_line();
                                    let original_column = original_location.get_src_col();
                                    log::trace!("lookup original_line:{original_line}");
                                    stack_trace_entry.line_number = Some(original_line+1);
                                    stack_trace_entry.column_number = Some(original_column+1);
                                }
                            }
                            Err(_) => {
                                log::trace!(
                                    "could not parse source_map for {}",
                                    stack_trace_entry.file_name.as_str()
                                );
                            }
                        }
                    }
                } else {
                    log::trace!("no map found for {}", stack_trace_entry.file_name.as_str());
                }

                // Now you have the original filename and line number
                // You can use them as needed
            }

            let ret = serialize_stack(&parsed_stack);
            log::trace!("fix_stack_trace ret:\n{ret}");
            ret
        }
        Err(_) => {
            log::error!("could not parse stack: \n{}", stack_trace);
            stack_trace.to_string()
        }
    }
}

#[cfg(test)]
pub mod tests {
    use crate::facades::tests::init_test_rt;
    use crate::jsutils::{JsValueType, Script};
    use crate::typescript::{parse_stack_trace, serialize_stack};

    #[test]
    fn test_ts() {
        let rt = init_test_rt();
        println!("testing ts");
        let script = Script::new(
            "test.ts",
            r#"
            // hi
            // ho
            function t_ts(a: string, b: num): boolean {
                return true;
            }
            t_ts("hello", 1337);
        "#,
        );
        let res = rt.eval_sync(None, script).expect("script failed");
        assert!(res.get_value_type() == JsValueType::Boolean);
    }
    #[test]
    fn test_stack_map() {
        let rt = init_test_rt();
        println!("testing ts");
        let script = Script::new(
            "test.ts",
            r#"
            
            type Nonsense = {
                hello: string
            };
            
            function t_ts(a: string, b: num): boolean {
                return a.a.a === "hi";
            }

            t_ts("hello", 1337);
"#,
        );
        let res = rt
            .eval_sync(None, script)
            .expect_err("script passed.. which it shouldnt");
        // far from perfect test, also line numbers don't yet realy match..
        // check again after https://github.com/HiRoFa/quickjs_es_runtime/issues/77
        println!("stack:{}",res.get_stack());
        assert!(res.get_stack().contains("t_ts (test.ts:7"));
    }
    #[test]
    fn test_stack_parse() {
        // just to init logging;
        let _rt = init_test_rt();

        let stack = r#"
            at func (file.ts:88:12)
            at doWriteTransactioned (gcsproject:///gcs_objectstore/ObjectStore.ts:170)
        "#;
        match parse_stack_trace(stack) {
            Ok(a) => {
                assert_eq!(a[0].file_name, "file.ts");
                assert_eq!(a[0].line_number, Some(88));
                assert_eq!(a[0].column_number, Some(12));
                assert_eq!(a[0].function_name, "func");

                assert_eq!(
                    a[1].file_name,
                    "gcsproject:///gcs_objectstore/ObjectStore.ts"
                );
                assert_eq!(a[1].line_number, Some(170));
                assert_eq!(a[1].column_number, None);
                assert_eq!(a[1].function_name, "doWriteTransactioned");

                assert_eq!(
                    serialize_stack(&a).as_str(),
                    r#"    at func (file.ts:88:12)
    at doWriteTransactioned (gcsproject:///gcs_objectstore/ObjectStore.ts:170)
"#
                );
            }
            Err(e) => {
                panic!("{}", e);
            }
        }
    }
}
