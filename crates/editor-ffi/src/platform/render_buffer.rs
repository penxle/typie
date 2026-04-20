use cfg_if::cfg_if;
use std::cell::UnsafeCell;
use std::sync::atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering};

const NONE: usize = usize::MAX;

pub struct Slot {
    pub data: Vec<u8>,
    pub width: u32,
    pub height: u32,
}

impl Slot {
    fn new(width: u32, height: u32) -> Self {
        let size = (width as usize) * (height as usize) * 4;
        Self {
            data: vec![0u8; size],
            width,
            height,
        }
    }

    fn ensure_size(&mut self, width: u32, height: u32) {
        let size = (width as usize) * (height as usize) * 4;
        if self.data.len() != size {
            self.data.resize(size, 0);
        }
        self.width = width;
        self.height = height;
    }
}

/// Lock-free triple buffer for single-producer / single-consumer frame handoff. Writer commits to
/// a slot that is neither `latest` nor `reading`; reader pins `latest` for the duration of a copy.
/// Because one slot is always free, writer never waits on reader and vice versa.
pub struct RenderBuffer {
    slots: [UnsafeCell<Slot>; 3],
    /// Index of the most-recently committed slot (0..3).
    latest: AtomicUsize,
    /// Slot currently pinned by the reader (0..3), or `NONE`.
    reading: AtomicUsize,
    /// Packed (width << 32) | height. Writer reads on commit to lazily resize the target slot.
    dims: AtomicU64,
    dirty: AtomicBool,
}

// UnsafeCell is !Sync by default. Triple-buffer invariants guarantee reader and writer access
// disjoint slots, so cross-thread access is safe.
unsafe impl Sync for RenderBuffer {}

impl RenderBuffer {
    pub fn new(width: u32, height: u32) -> Self {
        Self {
            slots: [
                UnsafeCell::new(Slot::new(width, height)),
                UnsafeCell::new(Slot::new(width, height)),
                UnsafeCell::new(Slot::new(width, height)),
            ],
            latest: AtomicUsize::new(0),
            reading: AtomicUsize::new(NONE),
            dims: AtomicU64::new(((width as u64) << 32) | (height as u64)),
            dirty: AtomicBool::new(false),
        }
    }

    /// Writer path: update target dimensions. Actual slot resize happens lazily on `commit`.
    pub fn resize(&self, width: u32, height: u32) {
        self.dims
            .store(((width as u64) << 32) | (height as u64), Ordering::Release);
    }

    /// Writer path: flush a new frame into a free slot and publish it as `latest`.
    pub fn commit<F>(&self, flush: F)
    where
        F: FnOnce(&mut [u8]),
    {
        let latest = self.latest.load(Ordering::Acquire);
        let reading = self.reading.load(Ordering::Acquire);
        let write_idx = (0..3)
            .find(|&i| i != latest && i != reading)
            .expect("triple buffer invariant: at least one slot must be free");

        let slot = unsafe { &mut *self.slots[write_idx].get() };
        let dims = self.dims.load(Ordering::Acquire);
        let w = (dims >> 32) as u32;
        let h = dims as u32;
        slot.ensure_size(w, h);
        flush(&mut slot.data);

        self.latest.store(write_idx, Ordering::Release);
        self.dirty.store(true, Ordering::Release);
    }

    /// Reader path: consume the dirty flag and pin the latest slot. Returns `true` if a new frame
    /// is available; caller must later call `end_read` to release the pin.
    pub fn begin_read(&self) -> bool {
        if !self.dirty.swap(false, Ordering::AcqRel) {
            return false;
        }
        loop {
            let latest = self.latest.load(Ordering::Acquire);
            self.reading.store(latest, Ordering::Release);
            // If writer committed a new frame between our load and store, our pin may have been
            // decided before writer saw it. Retry so writer sees the pin before selecting its
            // next write slot.
            if self.latest.load(Ordering::Acquire) == latest {
                return true;
            }
        }
    }

    /// Reader path: release the pin.
    pub fn end_read(&self) {
        self.reading.store(NONE, Ordering::Release);
    }

    fn pinned_slot(&self) -> Option<&Slot> {
        let idx = self.reading.load(Ordering::Acquire);
        if idx >= 3 {
            return None;
        }
        Some(unsafe { &*self.slots[idx].get() })
    }
}

cfg_if! {
    if #[cfg(target_os = "ios")] {
        #[unsafe(no_mangle)]
        pub extern "C" fn render_buffer_allocate(width: i32, height: i32) -> i64 {
            Box::into_raw(Box::new(RenderBuffer::new(width as u32, height as u32))) as i64
        }

        #[unsafe(no_mangle)]
        pub extern "C" fn render_buffer_free(handle: i64) {
            if handle != 0 {
                unsafe {
                    let _ = Box::from_raw(handle as *mut RenderBuffer);
                }
            }
        }

        #[unsafe(no_mangle)]
        pub extern "C" fn render_buffer_resize(handle: i64, width: i32, height: i32) {
            if handle == 0 {
                return;
            }
            unsafe { (*(handle as *const RenderBuffer)).resize(width as u32, height as u32) };
        }

        #[unsafe(no_mangle)]
        pub extern "C" fn render_buffer_begin_read(handle: i64) -> bool {
            if handle == 0 {
                return false;
            }
            unsafe { (*(handle as *const RenderBuffer)).begin_read() }
        }

        #[unsafe(no_mangle)]
        pub extern "C" fn render_buffer_end_read(handle: i64) {
            if handle == 0 {
                return;
            }
            unsafe { (*(handle as *const RenderBuffer)).end_read() }
        }

        #[unsafe(no_mangle)]
        pub extern "C" fn render_buffer_data_pointer(handle: i64) -> i64 {
            if handle == 0 {
                return 0;
            }
            unsafe {
                (*(handle as *const RenderBuffer))
                    .pinned_slot()
                    .map(|s| s.data.as_ptr() as i64)
                    .unwrap_or(0)
            }
        }

        #[unsafe(no_mangle)]
        pub extern "C" fn render_buffer_width(handle: i64) -> i32 {
            if handle == 0 {
                return 0;
            }
            unsafe {
                (*(handle as *const RenderBuffer))
                    .pinned_slot()
                    .map(|s| s.width as i32)
                    .unwrap_or(0)
            }
        }

        #[unsafe(no_mangle)]
        pub extern "C" fn render_buffer_height(handle: i64) -> i32 {
            if handle == 0 {
                return 0;
            }
            unsafe {
                (*(handle as *const RenderBuffer))
                    .pinned_slot()
                    .map(|s| s.height as i32)
                    .unwrap_or(0)
            }
        }
    } else {
        use std::ffi::c_void;

        #[unsafe(no_mangle)]
        pub extern "C" fn Java_co_typie_editor_render_RenderBuffer_allocate(
            _env: *mut c_void,
            _class: *mut c_void,
            width: i32,
            height: i32,
        ) -> i64 {
            Box::into_raw(Box::new(RenderBuffer::new(width as u32, height as u32))) as i64
        }

        #[unsafe(no_mangle)]
        pub extern "C" fn Java_co_typie_editor_render_RenderBuffer_free(
            _env: *mut c_void,
            _class: *mut c_void,
            handle: i64,
        ) {
            if handle != 0 {
                unsafe {
                    let _ = Box::from_raw(handle as *mut RenderBuffer);
                }
            }
        }

        #[unsafe(no_mangle)]
        pub extern "C" fn Java_co_typie_editor_render_RenderBuffer_resize(
            _env: *mut c_void,
            _class: *mut c_void,
            handle: i64,
            width: i32,
            height: i32,
        ) {
            if handle == 0 {
                return;
            }
            unsafe { (*(handle as *const RenderBuffer)).resize(width as u32, height as u32) };
        }

        #[unsafe(no_mangle)]
        pub extern "C" fn Java_co_typie_editor_render_RenderBuffer_beginRead(
            _env: *mut c_void,
            _class: *mut c_void,
            handle: i64,
        ) -> bool {
            if handle == 0 {
                return false;
            }
            unsafe { (*(handle as *const RenderBuffer)).begin_read() }
        }

        #[unsafe(no_mangle)]
        pub extern "C" fn Java_co_typie_editor_render_RenderBuffer_endRead(
            _env: *mut c_void,
            _class: *mut c_void,
            handle: i64,
        ) {
            if handle == 0 {
                return;
            }
            unsafe { (*(handle as *const RenderBuffer)).end_read() }
        }

        #[unsafe(no_mangle)]
        pub extern "C" fn Java_co_typie_editor_render_RenderBuffer_getDataPointer(
            _env: *mut c_void,
            _class: *mut c_void,
            handle: i64,
        ) -> i64 {
            if handle == 0 {
                return 0;
            }
            unsafe {
                (*(handle as *const RenderBuffer))
                    .pinned_slot()
                    .map(|s| s.data.as_ptr() as i64)
                    .unwrap_or(0)
            }
        }

        #[unsafe(no_mangle)]
        pub extern "C" fn Java_co_typie_editor_render_RenderBuffer_getPixelWidth(
            _env: *mut c_void,
            _class: *mut c_void,
            handle: i64,
        ) -> i32 {
            if handle == 0 {
                return 0;
            }
            unsafe {
                (*(handle as *const RenderBuffer))
                    .pinned_slot()
                    .map(|s| s.width as i32)
                    .unwrap_or(0)
            }
        }

        #[unsafe(no_mangle)]
        pub extern "C" fn Java_co_typie_editor_render_RenderBuffer_getPixelHeight(
            _env: *mut c_void,
            _class: *mut c_void,
            handle: i64,
        ) -> i32 {
            if handle == 0 {
                return 0;
            }
            unsafe {
                (*(handle as *const RenderBuffer))
                    .pinned_slot()
                    .map(|s| s.height as i32)
                    .unwrap_or(0)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn commit_publishes_frame_visible_to_begin_read() {
        let rb = RenderBuffer::new(2, 2);
        rb.commit(|data| data.fill(0xAB));
        assert!(rb.begin_read());
        let slot = rb.pinned_slot().unwrap();
        assert!(slot.data.iter().all(|b| *b == 0xAB));
        rb.end_read();
    }

    #[test]
    fn begin_read_returns_false_when_not_dirty() {
        let rb = RenderBuffer::new(1, 1);
        assert!(!rb.begin_read());
    }

    #[test]
    fn writer_avoids_reading_slot() {
        let rb = RenderBuffer::new(1, 1);
        rb.commit(|data| data[0] = 0x11);
        assert!(rb.begin_read());
        let reader_idx = rb.reading.load(Ordering::Acquire);
        rb.commit(|data| data[0] = 0x22);
        let pinned = rb.pinned_slot().unwrap();
        assert_eq!(pinned.data[0], 0x11);
        let new_latest = rb.latest.load(Ordering::Acquire);
        assert_ne!(new_latest, reader_idx);
        rb.end_read();
    }

    #[test]
    fn resize_affects_next_commit() {
        let rb = RenderBuffer::new(1, 1);
        rb.resize(3, 2);
        rb.commit(|data| assert_eq!(data.len(), 24));
        assert!(rb.begin_read());
        let slot = rb.pinned_slot().unwrap();
        assert_eq!(slot.width, 3);
        assert_eq!(slot.height, 2);
        rb.end_read();
    }
}
