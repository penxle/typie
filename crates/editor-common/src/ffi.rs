pub trait Ffi {
    type Target;
    type Error;

    fn to_ffi(&self) -> Self::Target;
    fn from_ffi(value: Self::Target) -> Result<Self, Self::Error>
    where
        Self: Sized;
}
