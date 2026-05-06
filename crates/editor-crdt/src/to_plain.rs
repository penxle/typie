/// Lossy projection of a CRDT wrapper to its alive/winner state.
/// The reverse (plain → CRDT) is not provided here; reconstruction
/// is an explicit, separate step.
pub trait ToPlain {
    type Plain;
    fn to_plain(&self) -> Self::Plain;
}
