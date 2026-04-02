use cfg_if::cfg_if;

use crate::error::FfiError;

pub trait IntoFfi {
    type Repr;
    fn into_ffi(self) -> Result<Self::Repr, FfiError>;
}

pub trait FromFfi {
    type Repr;
    fn from_ffi(self) -> Result<Self::Repr, FfiError>;
}

cfg_if! {
    if #[cfg(feature = "uniffi")] {
        impl<T> IntoFfi for T {
            type Repr = T;
            fn into_ffi(self) -> Result<T, FfiError> { Ok(self) }
        }

        impl<T> FromFfi for T {
            type Repr = T;
            fn from_ffi(self) -> Result<T, FfiError> { Ok(self) }
        }
    } else if #[cfg(feature = "wasm")] {
        impl<T> IntoFfi for T
        where
            T: tsify::Tsify + serde::Serialize,
        {
            type Repr = tsify::Ts<T>;
            fn into_ffi(self) -> Result<tsify::Ts<T>, FfiError> {
                tsify::Ts::from_rust(&self).map_err(|e| FfiError::Serialization(format!("{e:?}")))
            }
        }

        impl<T> FromFfi for tsify::Ts<T>
        where
            T: tsify::Tsify + serde::de::DeserializeOwned,
            <T as tsify::Tsify>::JsType: Clone,
        {
            type Repr = T;
            fn from_ffi(self) -> Result<T, FfiError> {
                self.to_rust().map_err(|e| FfiError::Deserialization(format!("{e:?}")))
            }
        }
    } else {
        impl<T> IntoFfi for T {
            type Repr = T;
            fn into_ffi(self) -> Result<T, FfiError> { Ok(self) }
        }

        impl<T> FromFfi for T {
            type Repr = T;
            fn from_ffi(self) -> Result<T, FfiError> { Ok(self) }
        }
    }
}

#[cfg(feature = "wasm")]
mod ext {
    use super::*;

    pub trait IntoFfiExt {
        type Repr;
        fn into_ffi(self) -> Result<Self::Repr, FfiError>;
    }

    pub trait FromFfiExt {
        type Repr;
        fn from_ffi(self) -> Result<Self::Repr, FfiError>;
    }

    impl<T: IntoFfi> IntoFfiExt for Vec<T> {
        type Repr = Vec<T::Repr>;
        fn into_ffi(self) -> Result<Vec<T::Repr>, FfiError> {
            self.into_iter().map(IntoFfi::into_ffi).collect()
        }
    }

    impl<T: FromFfi> FromFfiExt for Vec<T> {
        type Repr = Vec<T::Repr>;
        fn from_ffi(self) -> Result<Vec<T::Repr>, FfiError> {
            self.into_iter().map(FromFfi::from_ffi).collect()
        }
    }

    impl<T: IntoFfi> IntoFfiExt for Option<T> {
        type Repr = Option<T::Repr>;
        fn into_ffi(self) -> Result<Option<T::Repr>, FfiError> {
            self.map(IntoFfi::into_ffi).transpose()
        }
    }

    impl<T: FromFfi> FromFfiExt for Option<T> {
        type Repr = Option<T::Repr>;
        fn from_ffi(self) -> Result<Option<T::Repr>, FfiError> {
            self.map(FromFfi::from_ffi).transpose()
        }
    }
}

#[cfg(feature = "wasm")]
pub use ext::*;

#[cfg(feature = "wasm")]
impl From<crate::error::EditorError> for wasm_bindgen::JsValue {
    fn from(e: crate::error::EditorError) -> Self {
        wasm_bindgen::JsError::new(&e.to_string()).into()
    }
}
