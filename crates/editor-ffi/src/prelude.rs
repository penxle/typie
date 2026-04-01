use cfg_if::cfg_if;

pub use crate::error::*;
pub use crate::types::*;

cfg_if! {
    if #[cfg(feature = "uniffi")] {
        pub type Owned<T> = std::sync::Arc<T>;
        pub type Complex<T> = T;

        pub fn into_owned<T>(val: T) -> std::sync::Arc<T> { std::sync::Arc::new(val) }
    } else if #[cfg(feature = "wasm")] {
        pub type Owned<T> = T;
        pub type Complex<T> = tsify::Ts<T>;

        pub fn into_owned<T>(val: T) -> T { val }
    } else {
        pub type Owned<T> = T;
        pub type Complex<T> = T;

        pub fn into_owned<T>(val: T) -> T { val }
    }
}

pub type EditorResult<T> = Result<T, EditorError>;
