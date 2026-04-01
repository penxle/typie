#[cfg(target_os = "android")]
pub mod android;
#[cfg(not(any(target_os = "android", target_os = "ios")))]
pub mod desktop;
#[cfg(target_os = "ios")]
pub mod ios;
