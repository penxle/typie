pub trait StrExt {
    fn char_count(&self) -> usize;
    fn nth_char_byte_offset(&self, n: usize) -> usize;
    fn nth_byte_char_offset(&self, byte_offset: usize) -> usize;
}

impl StrExt for str {
    fn char_count(&self) -> usize {
        bytecount::num_chars(self.as_bytes())
    }

    fn nth_char_byte_offset(&self, n: usize) -> usize {
        bytecount::byte_offset_of_char(self.as_bytes(), n)
    }

    fn nth_byte_char_offset(&self, byte_offset: usize) -> usize {
        bytecount::num_chars(
            self.as_bytes()
                .get(..byte_offset)
                .unwrap_or(self.as_bytes()),
        )
    }
}
