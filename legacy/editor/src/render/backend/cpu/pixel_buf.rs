/// Premultiplied RGBA8 pixel buffer — replaces tiny_skia::Pixmap.
#[derive(Clone)]
pub struct PixelBuf {
    data: Vec<u8>,
    width: u32,
    height: u32,
}

impl PixelBuf {
    pub fn new(width: u32, height: u32) -> Option<Self> {
        if width == 0 || height == 0 {
            return None;
        }
        let len = (width as usize)
            .checked_mul(height as usize)?
            .checked_mul(4)?;
        Some(Self {
            data: vec![0u8; len],
            width,
            height,
        })
    }

    /// Create a borrowed mutable view over an existing byte slice.
    /// Returns None if dimensions are invalid or the slice is too small.
    pub fn from_bytes(data: &mut [u8], width: u32, height: u32) -> Option<PixelBufMut<'_>> {
        if width == 0 || height == 0 {
            return None;
        }
        let expected = width as usize * height as usize * 4;
        if data.len() < expected {
            return None;
        }
        Some(PixelBufMut {
            data,
            width,
            height,
        })
    }

    pub fn width(&self) -> u32 {
        self.width
    }

    pub fn height(&self) -> u32 {
        self.height
    }

    pub fn data(&self) -> &[u8] {
        &self.data
    }

    pub fn data_mut(&mut self) -> &mut [u8] {
        &mut self.data
    }
}

/// Non-owning mutable reference to a pixel buffer (replaces tiny_skia::PixmapMut).
pub struct PixelBufMut<'a> {
    data: &'a mut [u8],
    width: u32,
    height: u32,
}

impl<'a> PixelBufMut<'a> {
    /// Create a PixelBufMut from a mutable byte slice and dimensions.
    pub fn from_slice(data: &'a mut [u8], width: u32, height: u32) -> Self {
        Self {
            data,
            width,
            height,
        }
    }

    pub fn width(&self) -> u32 {
        self.width
    }

    pub fn height(&self) -> u32 {
        self.height
    }

    pub fn data_mut(&mut self) -> &mut [u8] {
        self.data
    }
}
