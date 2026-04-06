use std::cell::UnsafeCell;

// Safety: Write access (`as_mut_slice`) must only be called when no concurrent
// reads are possible. `FontRegistry` guarantees this by requiring `&mut self`
// for chunk application and `&self` for reads — Rust's borrow rules
// enforce exclusivity.
pub struct FontData(UnsafeCell<Vec<u8>>);

// Safety: FontRegistry controls all access. Write requires &mut FontRegistry,
// read requires &FontRegistry. Rust's borrow checker prevents concurrent access.
unsafe impl Send for FontData {}
unsafe impl Sync for FontData {}

impl FontData {
    pub fn new(data: Vec<u8>) -> Self {
        Self(UnsafeCell::new(data))
    }

    // Safety: Caller must ensure no shared references to the data exist.
    pub(crate) fn as_mut_ptr(&self) -> *mut [u8] {
        unsafe { (*self.0.get()).as_mut_slice() }
    }
}

impl AsRef<[u8]> for FontData {
    fn as_ref(&self) -> &[u8] {
        unsafe { &*self.0.get() }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn read_data() {
        let fd = FontData::new(vec![1, 2, 3]);
        assert_eq!(fd.as_ref(), &[1, 2, 3]);
    }

    #[test]
    fn mutate_data() {
        let fd = FontData::new(vec![0, 0, 0]);
        unsafe {
            (*fd.as_mut_ptr())[1] = 42;
        }
        assert_eq!(fd.as_ref(), &[0, 42, 0]);
    }
}
