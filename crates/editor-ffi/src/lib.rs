// ! When modifying this file, update the following:
// !   - apps/mobile2/compose/src/commonMain/kotlin/co/typie/editor/Editor.kt    (interface)
// !   - apps/mobile2/compose/src/jnaMain/kotlin/co/typie/editor/Editor.jna.kt   (Android + Desktop)
// !   - apps/mobile2/compose/src/iosMain/kotlin/co/typie/editor/Editor.ios.kt   (iOS)
// !   - apps/mobile2/ios/Bridge/Sources/Bridge/Editor/Editor.swift              (Swift @objc bridge)

editor_macros::preamble!();

#[cfg(all(feature = "uniffi", feature = "wasm"))]
compile_error!("features \"uniffi\" and \"wasm\" are mutually exclusive");

#[cfg(all(feature = "wasm-browser", feature = "wasm-server"))]
compile_error!("features \"wasm-browser\" and \"wasm-server\" are mutually exclusive");

mod convert;
pub mod editor;
mod error;
pub mod host;
#[cfg(not(feature = "wasm-server"))]
mod platform;
mod prelude;
#[cfg(feature = "wasm-server")]
mod server;
