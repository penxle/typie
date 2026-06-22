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
