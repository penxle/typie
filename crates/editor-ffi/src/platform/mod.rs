use cfg_if::cfg_if;

cfg_if! {
    if #[cfg(any(target_os = "android", target_os = "ios", feature = "uniffi"))] {
        mod render_buffer;
        mod cpu_surface;
        pub use cpu_surface::{PlatformHandle, SurfaceHandle};
    } else if #[cfg(feature = "wasm-browser")] {
        mod wasm_browser;
        pub use wasm_browser::{PlatformHandle, SurfaceHandle};
    } else {
        mod default;
        #[allow(unused_imports)]
        pub use default::{PlatformHandle, SurfaceHandle};
    }
}

#[cfg(all(
    test,
    not(any(target_os = "android", target_os = "ios", feature = "uniffi"))
))]
mod render_buffer;

#[cfg(any(
    test,
    target_os = "android",
    target_os = "ios",
    feature = "uniffi",
    feature = "wasm-browser"
))]
mod tiled_surface;

#[cfg(all(
    test,
    not(any(
        target_os = "android",
        target_os = "ios",
        feature = "uniffi",
        feature = "wasm-browser"
    ))
))]
mod cpu_surface;
