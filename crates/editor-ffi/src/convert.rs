use crate::error::FfiError;

pub trait FromFfi<T> {
    fn from_ffi(self) -> Result<T, FfiError>;
}

pub trait IntoFfi<T> {
    fn into_ffi(self) -> Result<T, FfiError>;
}

macro_rules! impl_ffi_identity {
    ($($ty:ty),*) => {
        $(
            impl FromFfi<$ty> for $ty {
                fn from_ffi(self) -> Result<$ty, FfiError> { Ok(self) }
            }

            impl IntoFfi<$ty> for $ty {
                fn into_ffi(self) -> Result<$ty, FfiError> { Ok(self) }
            }
        )*
    };
}

impl_ffi_identity!(
    bool, u8, u16, u32, u64, i8, i16, i32, i64, f32, f64, usize, String
);

impl<F, C> FromFfi<Option<C>> for Option<F>
where
    F: FromFfi<C>,
{
    fn from_ffi(self) -> Result<Option<C>, FfiError> {
        self.map(FromFfi::from_ffi).transpose()
    }
}

impl<F, C> IntoFfi<Option<C>> for Option<F>
where
    F: IntoFfi<C>,
{
    fn into_ffi(self) -> Result<Option<C>, FfiError> {
        self.map(IntoFfi::into_ffi).transpose()
    }
}

impl<F, C> FromFfi<Vec<C>> for Vec<F>
where
    F: FromFfi<C>,
{
    fn from_ffi(self) -> Result<Vec<C>, FfiError> {
        self.into_iter().map(FromFfi::from_ffi).collect()
    }
}

impl<F, C> IntoFfi<Vec<C>> for Vec<F>
where
    F: IntoFfi<C>,
{
    fn into_ffi(self) -> Result<Vec<C>, FfiError> {
        self.into_iter().map(IntoFfi::into_ffi).collect()
    }
}

#[cfg(feature = "uniffi")]
uniffi::custom_type!(usize, u64, {
    remote,
    lower: |obj| obj as u64,
    try_lift: |val| Ok(val as usize),
});

#[cfg(feature = "wasm")]
impl<T: tsify::Tsify + serde::de::DeserializeOwned + FromFfi<U>, U> FromFfi<U> for tsify::Ts<T>
where
    <T as tsify::Tsify>::JsType: Clone,
{
    fn from_ffi(self) -> Result<U, FfiError> {
        let val = self
            .to_rust()
            .map_err(|e| FfiError::Deserialization(format!("{e:?}")))?;
        val.from_ffi()
    }
}

#[cfg(feature = "wasm")]
impl<T: tsify::Tsify + serde::Serialize, U: IntoFfi<T>> IntoFfi<tsify::Ts<T>> for U {
    fn into_ffi(self) -> Result<tsify::Ts<T>, FfiError> {
        let ffi = IntoFfi::into_ffi(self)?;
        tsify::Ts::from_rust(&ffi).map_err(|e| FfiError::Serialization(format!("{e:?}")))
    }
}

#[cfg(feature = "wasm")]
impl From<crate::error::EditorError> for wasm_bindgen::JsValue {
    fn from(e: crate::error::EditorError) -> Self {
        wasm_bindgen::JsError::new(&e.to_string()).into()
    }
}
