use cfg_if::cfg_if;

cfg_if! {
    if #[cfg(target_os = "android")] {
        mod android;
        pub use android::{PlatformHandle, SurfaceHandle};
    } else if #[cfg(target_os = "ios")] {
        mod ios;
        pub use ios::{PlatformHandle, SurfaceHandle};
    } else if #[cfg(feature = "wasm-browser")] {
        mod wasm_browser;
        pub use wasm_browser::{PlatformHandle, SurfaceHandle};
    } else {
        mod desktop;
        pub use desktop::{PlatformHandle, SurfaceHandle};
    }
}
