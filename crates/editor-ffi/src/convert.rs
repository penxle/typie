use cfg_if::cfg_if;

use crate::error::FfiError;

#[allow(clippy::wrong_self_convention)]
pub trait FromFfi<T> {
    fn from_ffi(self) -> Result<T, FfiError>;
}

pub trait IntoFfi<R> {
    fn into_ffi(self) -> Result<R, FfiError>;
}

cfg_if! {
    if #[cfg(feature = "uniffi")] {
        impl<T: serde::de::DeserializeOwned> FromFfi<T> for String {
            fn from_ffi(self) -> Result<T, FfiError> {
                serde_json::from_str(&self)
                    .map_err(|e| FfiError::Deserialization(e.to_string()))
            }
        }

        impl<T: serde::Serialize> IntoFfi<String> for T {
            fn into_ffi(self) -> Result<String, FfiError> {
                serde_json::to_string(&self)
                    .map_err(|e| FfiError::Serialization(e.to_string()))
            }
        }
    } else if #[cfg(feature = "wasm")] {
        impl<T> IntoFfi<tsify::Ts<T>> for T
        where
            T: tsify::Tsify + serde::Serialize,
        {
            fn into_ffi(self) -> Result<tsify::Ts<T>, FfiError> {
                tsify::Ts::from_rust(&self).map_err(|e| FfiError::Serialization(format!("{e:?}")))
            }
        }

        impl<T> FromFfi<T> for tsify::Ts<T>
        where
            T: tsify::Tsify + serde::de::DeserializeOwned,
            <T as tsify::Tsify>::JsType: Clone,
        {
            fn from_ffi(self) -> Result<T, FfiError> {
                self.to_rust().map_err(|e| FfiError::Deserialization(format!("{e:?}")))
            }
        }
    } else {
        impl<T> FromFfi<T> for T {
            fn from_ffi(self) -> Result<T, FfiError> { Ok(self) }
        }

        impl<T> IntoFfi<T> for T {
            fn into_ffi(self) -> Result<T, FfiError> { Ok(self) }
        }
    }
}

// Container delegation — only needed when blanket T→T impls are absent
cfg_if! {
    if #[cfg(any(feature = "uniffi", feature = "wasm"))] {
        impl<T, R> IntoFfi<Vec<R>> for Vec<T>
        where
            T: IntoFfi<R>,
        {
            fn into_ffi(self) -> Result<Vec<R>, FfiError> {
                self.into_iter().map(IntoFfi::into_ffi).collect()
            }
        }

        impl<T, R> IntoFfi<Option<R>> for Option<T>
        where
            T: IntoFfi<R>,
        {
            fn into_ffi(self) -> Result<Option<R>, FfiError> {
                self.map(IntoFfi::into_ffi).transpose()
            }
        }

        impl<T, R> FromFfi<Vec<R>> for Vec<T>
        where
            T: FromFfi<R>,
        {
            fn from_ffi(self) -> Result<Vec<R>, FfiError> {
                self.into_iter().map(FromFfi::from_ffi).collect()
            }
        }

        impl<T, R> FromFfi<Option<R>> for Option<T>
        where
            T: FromFfi<R>,
        {
            fn from_ffi(self) -> Result<Option<R>, FfiError> {
                self.map(FromFfi::from_ffi).transpose()
            }
        }
    }
}

#[cfg(feature = "wasm")]
impl From<crate::error::EditorError> for wasm_bindgen::JsValue {
    fn from(e: crate::error::EditorError) -> Self {
        wasm_bindgen::JsError::new(&e.to_string()).into()
    }
}
