use object::{Object, ObjectSection, ObjectSymbol, ObjectSymbolTable};
use std::collections::BTreeMap;
use std::path::Path;

use crate::meta::{FfiInterface, FfiMeta};

pub fn read_ffi_meta(path: &Path) -> Vec<FfiMeta> {
    let data = std::fs::read(path).expect("failed to read binary");
    let file = object::File::parse(&*data).expect("failed to parse binary");
    let mut result = Vec::new();

    if let Some(symbol_table) = file.symbol_table() {
        for symbol in symbol_table.symbols() {
            let name = match symbol.name() {
                Ok(n) => n,
                Err(_) => continue,
            };

            if !name.starts_with("FFI_META_") && !name.starts_with("_FFI_META_") {
                continue;
            }

            let address = symbol.address();

            if let Some(section_index) = symbol.section_index()
                && let Ok(section) = file.section_by_index(section_index)
                && let Ok(section_data) = section.data()
            {
                let section_addr = section.address();
                let offset = (address - section_addr) as usize;

                if offset + 4 > section_data.len() {
                    continue;
                }

                let payload_len =
                    u32::from_le_bytes(section_data[offset..offset + 4].try_into().unwrap())
                        as usize;

                let payload_start = offset + 4;
                if payload_start + payload_len > section_data.len() {
                    continue;
                }

                let bytes = &section_data[payload_start..payload_start + payload_len];

                match bitcode::decode::<FfiMeta>(bytes) {
                    Ok(meta) => result.push(meta),
                    Err(e) => eprintln!("warning: failed to decode {}: {}", name, e),
                }
            }
        }
    }

    result
}

pub fn read_ffi_interfaces(path: &Path) -> Vec<FfiInterface> {
    let data = std::fs::read(path).expect("failed to read binary");
    let file = object::File::parse(&*data).expect("failed to parse binary");
    let mut decoded = Vec::new();

    if let Some(symbol_table) = file.symbol_table() {
        for symbol in symbol_table.symbols() {
            let name = match symbol.name() {
                Ok(n) => n,
                Err(_) => continue,
            };

            if !name.starts_with("FFI_IFACE_") && !name.starts_with("_FFI_IFACE_") {
                continue;
            }

            let address = symbol.address();

            if let Some(section_index) = symbol.section_index()
                && let Ok(section) = file.section_by_index(section_index)
                && let Ok(section_data) = section.data()
            {
                let section_addr = section.address();
                let offset = (address - section_addr) as usize;

                if offset + 4 > section_data.len() {
                    continue;
                }

                let payload_len =
                    u32::from_le_bytes(section_data[offset..offset + 4].try_into().unwrap())
                        as usize;

                let payload_start = offset + 4;
                if payload_start + payload_len > section_data.len() {
                    continue;
                }

                let bytes = &section_data[payload_start..payload_start + payload_len];

                match bitcode::decode::<FfiInterface>(bytes) {
                    Ok(iface) => decoded.push(iface),
                    Err(e) => eprintln!("warning: failed to decode {}: {}", name, e),
                }
            }
        }
    }

    merge_by_name(decoded)
}

fn merge_by_name(interfaces: Vec<FfiInterface>) -> Vec<FfiInterface> {
    let mut by_name: BTreeMap<String, FfiInterface> = BTreeMap::new();

    for iface in interfaces {
        by_name
            .entry(iface.name.clone())
            .and_modify(|existing| {
                existing.methods.extend(iface.methods.clone());
            })
            .or_insert(iface);
    }

    by_name.into_values().collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::meta::*;

    fn method(name: &str) -> FfiMethod {
        FfiMethod {
            name: name.into(),
            is_async: false,
            is_constructor: false,
            params: vec![],
            return_type: FfiReturnType::Unit,
        }
    }

    fn iface(name: &str, methods: &[&str]) -> FfiInterface {
        FfiInterface {
            name: name.into(),
            methods: methods.iter().copied().map(method).collect(),
        }
    }

    #[test]
    fn merge_by_name_orders_interfaces_by_name() {
        let merged = merge_by_name(vec![
            iface("GraphIngest", &["ingest"]),
            iface("EditorHost", &["new"]),
            iface("Editor", &["root"]),
            iface("EditorFrame", &["width"]),
        ]);

        let names: Vec<_> = merged.iter().map(|i| i.name.as_str()).collect();
        assert_eq!(
            names,
            ["Editor", "EditorFrame", "EditorHost", "GraphIngest"]
        );
    }

    #[test]
    fn merge_by_name_appends_methods_in_input_order() {
        let merged = merge_by_name(vec![
            iface("Editor", &["root", "doc"]),
            iface("EditorHost", &["new"]),
            iface("Editor", &["selection"]),
            iface("EditorHost", &["create_editor"]),
            iface("Editor", &["undo", "redo"]),
        ]);

        let summary: Vec<(&str, Vec<&str>)> = merged
            .iter()
            .map(|i| {
                (
                    i.name.as_str(),
                    i.methods.iter().map(|m| m.name.as_str()).collect(),
                )
            })
            .collect();
        assert_eq!(
            summary,
            [
                ("Editor", vec!["root", "doc", "selection", "undo", "redo"]),
                ("EditorHost", vec!["new", "create_editor"]),
            ]
        );
    }
}
