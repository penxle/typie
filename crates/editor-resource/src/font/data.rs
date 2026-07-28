pub struct FontData(Vec<u8>);

impl FontData {
    pub fn new(data: Vec<u8>) -> Self {
        Self(data)
    }

    pub(crate) fn patched<'a>(&self, patches: impl IntoIterator<Item = (usize, &'a [u8])>) -> Self {
        let mut data = self.0.clone();
        for (offset, bytes) in patches {
            data[offset..offset + bytes.len()].copy_from_slice(bytes);
        }
        Self(data)
    }
}

impl AsRef<[u8]> for FontData {
    fn as_ref(&self) -> &[u8] {
        &self.0
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
        let patched = fd.patched([(1, &[42][..])]);
        assert_eq!(fd.as_ref(), &[0, 0, 0]);
        assert_eq!(patched.as_ref(), &[0, 42, 0]);
    }
}
