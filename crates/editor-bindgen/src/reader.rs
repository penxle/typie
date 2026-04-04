use object::{Object, ObjectSection, ObjectSymbol, ObjectSymbolTable};
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

            if let Some(section_index) = symbol.section_index() {
                if let Ok(section) = file.section_by_index(section_index) {
                    if let Ok(section_data) = section.data() {
                        let section_addr = section.address();
                        let offset = (address - section_addr) as usize;

                        if offset + 4 > section_data.len() {
                            continue;
                        }

                        let payload_len = u32::from_le_bytes(
                            section_data[offset..offset + 4].try_into().unwrap(),
                        ) as usize;

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
        }
    }

    result
}

pub fn read_ffi_interfaces(path: &Path) -> Vec<FfiInterface> {
    let data = std::fs::read(path).expect("failed to read binary");
    let file = object::File::parse(&*data).expect("failed to parse binary");
    let mut by_name: std::collections::HashMap<String, FfiInterface> =
        std::collections::HashMap::new();

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

            if let Some(section_index) = symbol.section_index() {
                if let Ok(section) = file.section_by_index(section_index) {
                    if let Ok(section_data) = section.data() {
                        let section_addr = section.address();
                        let offset = (address - section_addr) as usize;

                        if offset + 4 > section_data.len() {
                            continue;
                        }

                        let payload_len = u32::from_le_bytes(
                            section_data[offset..offset + 4].try_into().unwrap(),
                        ) as usize;

                        let payload_start = offset + 4;
                        if payload_start + payload_len > section_data.len() {
                            continue;
                        }

                        let bytes = &section_data[payload_start..payload_start + payload_len];

                        match bitcode::decode::<FfiInterface>(bytes) {
                            Ok(iface) => {
                                by_name
                                    .entry(iface.name.clone())
                                    .and_modify(|existing| {
                                        existing.methods.extend(iface.methods.clone());
                                    })
                                    .or_insert(iface);
                            }
                            Err(e) => eprintln!("warning: failed to decode {}: {}", name, e),
                        }
                    }
                }
            }
        }
    }

    by_name.into_values().collect()
}
