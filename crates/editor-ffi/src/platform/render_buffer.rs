use cfg_if::cfg_if;
use editor_renderer::damage::{IRect, merge_damage};
use std::cell::UnsafeCell;
use std::collections::VecDeque;
use std::sync::atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering};

const NONE: usize = usize::MAX;
const LOG_CAP: usize = 8;

const fn token(version: u64, index: usize) -> u64 {
    (version << 2) | index as u64
}

const fn token_index(t: u64) -> usize {
    (t & 0b11) as usize
}

const fn token_version(t: u64) -> u64 {
    t >> 2
}

pub struct Slot {
    pub data: Vec<u8>,
    pub width: u32,
    pub height: u32,
    pub version: u64,
    pub damage_from: u64,
    pub damage: Vec<i32>,
}

impl Slot {
    fn new(width: u32, height: u32) -> Self {
        let size = (width as usize) * (height as usize) * 4;
        Self {
            data: vec![0u8; size],
            width,
            height,
            version: 0,
            damage_from: 0,
            damage: Vec::new(),
        }
    }

    fn ensure_size(&mut self, width: u32, height: u32) -> bool {
        let size = (width as usize) * (height as usize) * 4;
        let resized = self.data.len() != size || self.width != width || self.height != height;
        if self.data.len() != size {
            self.data.resize(size, 0);
        }
        if resized {
            self.version = 0;
            self.damage_from = 0;
            self.damage.clear();
        }
        self.width = width;
        self.height = height;
        resized
    }
}

struct WriteGuard<'a>(&'a AtomicBool);

impl Drop for WriteGuard<'_> {
    fn drop(&mut self) {
        self.0.store(false, Ordering::Release);
    }
}

/// Lock-free triple buffer for single-producer / single-consumer frame handoff. Writer commits to
/// a slot that is neither `latest` nor `reading`; reader pins `latest` for the duration of a copy.
/// Because one slot is always free, writer never waits on reader and vice versa.
pub struct RenderBuffer {
    slots: [UnsafeCell<Slot>; 3],
    /// Versioned publish token `(frame_version << 2) | index`; only the writer stores it.
    latest: AtomicU64,
    /// Slot currently pinned by the reader (0..3), or `NONE`.
    reading: AtomicUsize,
    /// Packed (width << 32) | height. Writer reads on commit to lazily resize the target slot.
    dims: AtomicU64,
    dirty: AtomicBool,
    writing: AtomicBool,
    damage_log: UnsafeCell<VecDeque<(u64, Vec<IRect>)>>,
}

// # Safety
// `UnsafeCell` is `!Sync` by default. The writer selects `write_idx != reading` (and never the
// reader's pinned slot), so reader and writer touch disjoint slots; the token release/acquire pair
// publishes every slot field written before it. `damage_log` is writer-only, and `commit_damage`
// is single-producer and non-reentrant (enforced by the `writing` CAS guard), so the writer-only
// `UnsafeCell` is never aliased.
unsafe impl Sync for RenderBuffer {}

impl RenderBuffer {
    pub fn new(width: u32, height: u32) -> Self {
        Self {
            slots: [
                UnsafeCell::new(Slot::new(width, height)),
                UnsafeCell::new(Slot::new(width, height)),
                UnsafeCell::new(Slot::new(width, height)),
            ],
            latest: AtomicU64::new(token(0, 0)),
            reading: AtomicUsize::new(NONE),
            dims: AtomicU64::new(((width as u64) << 32) | (height as u64)),
            dirty: AtomicBool::new(false),
            writing: AtomicBool::new(false),
            damage_log: UnsafeCell::new(VecDeque::new()),
        }
    }

    /// Writer path: update target dimensions. Actual slot resize happens lazily on `commit_damage`.
    pub fn resize(&self, width: u32, height: u32) {
        self.dims
            .store(((width as u64) << 32) | (height as u64), Ordering::Release);
    }

    /// Writer path: copy this frame's damage into a free slot (catching the slot up from its stale
    /// version via the damage log) and publish it. Returns `false` without publishing if a
    /// concurrent producer holds the single-producer guard.
    pub fn commit_damage<F>(&self, this_frame_damage: &[IRect], mut copy_rect: F) -> bool
    where
        F: FnMut(&mut [u8], IRect),
    {
        if self
            .writing
            .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
            .is_err()
        {
            debug_assert!(
                false,
                "commit_damage is single-producer and must not overlap"
            );
            return false;
        }
        let _guard = WriteGuard(&self.writing);

        let latest_tok = self.latest.load(Ordering::Acquire);
        let cur_v = token_version(latest_tok);
        let nv = cur_v + 1;
        let latest_idx = token_index(latest_tok);
        let reading = self.reading.load(Ordering::Acquire);
        let write_idx = (0..3)
            .find(|&i| i != latest_idx && i != reading)
            .expect("triple buffer invariant: at least one slot must be free");

        let slot = unsafe { &mut *self.slots[write_idx].get() };
        let w_v = slot.version;

        let dims = self.dims.load(Ordering::Acquire);
        let w = (dims >> 32) as u32;
        let h = dims as u32;
        let resized = slot.ensure_size(w, h);

        let full_rect = IRect {
            x0: 0,
            y0: 0,
            x1: w as i32,
            y1: h as i32,
        };

        let log = unsafe { &mut *self.damage_log.get() };
        let oldest_log_version = log.front().map(|(v, _)| *v);

        let range_nonempty = w_v + 1 <= cur_v;
        let log_covers_range = match oldest_log_version {
            Some(oldest) => oldest <= w_v + 1,
            None => false,
        };
        let force_full = w_v == 0 || resized || (range_nonempty && !log_covers_range);

        let (catch, damage_from) = if force_full {
            (vec![full_rect], 0u64)
        } else {
            let mut acc: Vec<IRect> = log
                .iter()
                .filter(|(v, _)| *v >= w_v + 1 && *v <= cur_v)
                .flat_map(|(_, rs)| rs.iter().copied())
                .collect();
            acc.extend_from_slice(this_frame_damage);
            (merge_damage(&acc, full_rect), w_v)
        };

        for &r in &catch {
            copy_rect(&mut slot.data, r);
        }

        slot.version = nv;
        slot.damage_from = damage_from;
        slot.damage.clear();
        for r in &catch {
            slot.damage.extend_from_slice(&[r.x0, r.y0, r.x1, r.y1]);
        }

        log.push_back((nv, this_frame_damage.to_vec()));
        while log.len() > LOG_CAP {
            log.pop_front();
        }

        self.latest.store(token(nv, write_idx), Ordering::Release);
        self.dirty.store(true, Ordering::Release);
        true
    }

    /// Reader path: consume the dirty flag and pin the latest slot. Returns `true` if a new frame
    /// is available; caller must later call `end_read` to release the pin.
    pub fn begin_read(&self) -> bool {
        if !self.dirty.swap(false, Ordering::AcqRel) {
            return false;
        }
        loop {
            let latest = self.latest.load(Ordering::Acquire);
            self.reading.store(token_index(latest), Ordering::Release);
            // If writer committed a new frame between our load and store, retry so writer sees the
            // pin before selecting its next write slot. Token versions are monotonic (ABA-free).
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

    fn pinned_version(&self) -> u64 {
        self.pinned_slot().map(|s| s.version).unwrap_or(0)
    }

    fn pinned_damage_from(&self) -> u64 {
        self.pinned_slot().map(|s| s.damage_from).unwrap_or(0)
    }

    fn pinned_damage(&self) -> &[i32] {
        self.pinned_slot()
            .map(|s| s.damage.as_slice())
            .unwrap_or(&[])
    }

    /// # Safety
    /// `dst` must be valid for `dst_len` bytes of writes and must not overlap the pinned slot.
    pub unsafe fn read_pinned_into(
        &self,
        dst: *mut u8,
        dst_len: usize,
        row_from: u32,
        row_to: u32,
    ) -> bool {
        let Some(slot) = self.pinned_slot() else {
            return false;
        };
        if dst_len != slot.data.len() || row_from > row_to || row_to > slot.height {
            return false;
        }
        let stride = slot.width as usize * 4;
        let start = row_from as usize * stride;
        let end = row_to as usize * stride;
        unsafe {
            std::ptr::copy_nonoverlapping(
                slot.data[start..end].as_ptr(),
                dst.add(start),
                end - start,
            );
        }
        true
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

        // # Safety: the returned pointer is only valid between `render_buffer_begin_read` and
        // `render_buffer_end_read`; after `end_read` the writer may reclaim the pinned slot.
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

        #[unsafe(no_mangle)]
        pub extern "C" fn render_buffer_pinned_version(handle: i64) -> i64 {
            if handle == 0 {
                return 0;
            }
            unsafe { (*(handle as *const RenderBuffer)).pinned_version() as i64 }
        }

        #[unsafe(no_mangle)]
        pub extern "C" fn render_buffer_pinned_damage_from(handle: i64) -> i64 {
            if handle == 0 {
                return 0;
            }
            unsafe { (*(handle as *const RenderBuffer)).pinned_damage_from() as i64 }
        }

        // # Safety: the returned pointer is only valid between `render_buffer_begin_read` and
        // `render_buffer_end_read`; after `end_read` the writer may reclaim the pinned slot.
        #[unsafe(no_mangle)]
        pub extern "C" fn render_buffer_pinned_damage_pointer(handle: i64) -> i64 {
            if handle == 0 {
                return 0;
            }
            unsafe { (*(handle as *const RenderBuffer)).pinned_damage().as_ptr() as i64 }
        }

        #[unsafe(no_mangle)]
        pub extern "C" fn render_buffer_pinned_damage_count(handle: i64) -> i32 {
            if handle == 0 {
                return 0;
            }
            unsafe { ((*(handle as *const RenderBuffer)).pinned_damage().len() / 4) as i32 }
        }

        #[unsafe(no_mangle)]
        pub extern "C" fn render_buffer_read_pinned_into(
            handle: i64,
            dst: i64,
            dst_len: i64,
            row_from: i32,
            row_to: i32,
        ) -> bool {
            if handle == 0 || dst == 0 || dst_len < 0 || row_from < 0 || row_to < 0 {
                return false;
            }
            unsafe {
                (*(handle as *const RenderBuffer)).read_pinned_into(
                    dst as *mut u8,
                    dst_len as usize,
                    row_from as u32,
                    row_to as u32,
                )
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

        // # Safety: the returned pointer is only valid between `beginRead` and `endRead`; after
        // `endRead` the writer may reclaim the pinned slot.
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

        #[unsafe(no_mangle)]
        pub extern "C" fn Java_co_typie_editor_render_RenderBuffer_getPinnedVersion(
            _env: *mut c_void,
            _class: *mut c_void,
            handle: i64,
        ) -> i64 {
            if handle == 0 {
                return 0;
            }
            unsafe { (*(handle as *const RenderBuffer)).pinned_version() as i64 }
        }

        #[unsafe(no_mangle)]
        pub extern "C" fn Java_co_typie_editor_render_RenderBuffer_getPinnedDamageFrom(
            _env: *mut c_void,
            _class: *mut c_void,
            handle: i64,
        ) -> i64 {
            if handle == 0 {
                return 0;
            }
            unsafe { (*(handle as *const RenderBuffer)).pinned_damage_from() as i64 }
        }

        // # Safety: the returned pointer is only valid between `beginRead` and `endRead`; after
        // `endRead` the writer may reclaim the pinned slot.
        #[unsafe(no_mangle)]
        pub extern "C" fn Java_co_typie_editor_render_RenderBuffer_getPinnedDamagePointer(
            _env: *mut c_void,
            _class: *mut c_void,
            handle: i64,
        ) -> i64 {
            if handle == 0 {
                return 0;
            }
            unsafe { (*(handle as *const RenderBuffer)).pinned_damage().as_ptr() as i64 }
        }

        #[unsafe(no_mangle)]
        pub extern "C" fn Java_co_typie_editor_render_RenderBuffer_getPinnedDamageCount(
            _env: *mut c_void,
            _class: *mut c_void,
            handle: i64,
        ) -> i32 {
            if handle == 0 {
                return 0;
            }
            unsafe { ((*(handle as *const RenderBuffer)).pinned_damage().len() / 4) as i32 }
        }

        #[unsafe(no_mangle)]
        pub extern "C" fn Java_co_typie_editor_render_RenderBuffer_readPinnedInto(
            _env: *mut c_void,
            _class: *mut c_void,
            handle: i64,
            dst: i64,
            dst_len: i64,
            row_from: i32,
            row_to: i32,
        ) -> bool {
            if handle == 0 || dst == 0 || dst_len < 0 || row_from < 0 || row_to < 0 {
                return false;
            }
            unsafe {
                (*(handle as *const RenderBuffer)).read_pinned_into(
                    dst as *mut u8,
                    dst_len as usize,
                    row_from as u32,
                    row_to as u32,
                )
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use editor_renderer::damage::{IRect, merge_damage};

    fn full_rect(w: u32, h: u32) -> IRect {
        IRect {
            x0: 0,
            y0: 0,
            x1: w as i32,
            y1: h as i32,
        }
    }

    fn px(x: i32, y: i32) -> IRect {
        IRect {
            x0: x,
            y0: y,
            x1: x + 1,
            y1: y + 1,
        }
    }

    fn published_slot(rb: &RenderBuffer) -> &Slot {
        let idx = token_index(rb.latest.load(Ordering::Acquire));
        unsafe { &*rb.slots[idx].get() }
    }

    fn set_px(master: &mut [u8], stride: usize, x: i32, y: i32, val: u8) {
        let off = y as usize * stride + x as usize * 4;
        master[off..off + 4].fill(val);
    }

    fn copy_from_master(master: &[u8], stride: usize, data: &mut [u8], r: IRect) {
        for y in r.y0..r.y1 {
            for x in r.x0..r.x1 {
                let off = y as usize * stride + x as usize * 4;
                data[off..off + 4].copy_from_slice(&master[off..off + 4]);
            }
        }
    }

    #[test]
    fn commit_publishes_frame_visible_to_begin_read() {
        let rb = RenderBuffer::new(2, 2);
        assert!(rb.commit_damage(&[full_rect(2, 2)], |data, _r| data.fill(0xAB)));
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
        assert!(rb.commit_damage(&[full_rect(1, 1)], |data, _r| data[0] = 0x11));
        assert!(rb.begin_read());
        let reader_idx = rb.reading.load(Ordering::Acquire);
        assert!(rb.commit_damage(&[full_rect(1, 1)], |data, _r| data[0] = 0x22));
        let pinned = rb.pinned_slot().unwrap();
        assert_eq!(pinned.data[0], 0x11);
        let new_latest = rb.latest.load(Ordering::Acquire);
        assert_ne!(token_index(new_latest), reader_idx);
        rb.end_read();
    }

    #[test]
    fn resize_affects_next_commit() {
        let rb = RenderBuffer::new(1, 1);
        rb.resize(3, 2);
        assert!(rb.commit_damage(&[full_rect(3, 2)], |data, _r| assert_eq!(data.len(), 24)));
        assert!(rb.begin_read());
        let slot = rb.pinned_slot().unwrap();
        assert_eq!(slot.width, 3);
        assert_eq!(slot.height, 2);
        rb.end_read();
    }

    #[test]
    fn first_commit_is_full_and_bumps_version() {
        let rb = RenderBuffer::new(2, 2);
        let mut recorded = Vec::new();
        assert!(rb.commit_damage(&[px(0, 0)], |_d, r| recorded.push(r)));
        let tok = rb.latest.load(Ordering::Acquire);
        assert_eq!(token_version(tok), 1);
        let s = published_slot(&rb);
        assert_eq!(s.version, 1);
        assert_eq!(s.damage_from, 0);
        assert_eq!(s.damage, vec![0, 0, 2, 2]);
        assert_eq!(recorded, vec![full_rect(2, 2)]);
    }

    #[test]
    fn catch_up_reconstructs_current_frame() {
        let (w, h) = (4u32, 4u32);
        let stride = w as usize * 4;
        let rb = RenderBuffer::new(w, h);
        let mut master = vec![0u8; (w * h * 4) as usize];

        master.iter_mut().for_each(|b| *b = 0x10);
        assert!(
            rb.commit_damage(&[full_rect(w, h)], |d, r| copy_from_master(
                &master, stride, d, r
            ))
        );
        assert_eq!(published_slot(&rb).data, master);
        assert_eq!(published_slot(&rb).damage_from, 0);

        set_px(&mut master, stride, 0, 0, 0x20);
        assert!(rb.commit_damage(&[px(0, 0)], |d, r| copy_from_master(&master, stride, d, r)));
        assert_eq!(published_slot(&rb).data, master);

        set_px(&mut master, stride, 3, 3, 0x30);
        assert!(rb.commit_damage(&[px(3, 3)], |d, r| copy_from_master(&master, stride, d, r)));
        let s = published_slot(&rb);
        assert_eq!(s.data, master);
        assert_eq!(s.damage_from, 1);

        set_px(&mut master, stride, 1, 1, 0x40);
        assert!(rb.commit_damage(&[px(1, 1)], |d, r| copy_from_master(&master, stride, d, r)));
        let s = published_slot(&rb);
        assert_eq!(s.data, master);
        assert_eq!(s.damage_from, 2);
    }

    #[test]
    fn starved_slot_is_fully_reconstructed() {
        let (w, h) = (4u32, 4u32);
        let stride = w as usize * 4;
        let rb = RenderBuffer::new(w, h);
        let mut master = vec![0u8; (w * h * 4) as usize];

        master.iter_mut().for_each(|b| *b = 0x10);
        assert!(
            rb.commit_damage(&[full_rect(w, h)], |d, r| copy_from_master(
                &master, stride, d, r
            ))
        );

        assert!(rb.begin_read());

        set_px(&mut master, stride, 0, 0, 0x20);
        assert!(rb.commit_damage(&[px(0, 0)], |d, r| copy_from_master(&master, stride, d, r)));

        set_px(&mut master, stride, 3, 3, 0x30);
        assert!(rb.commit_damage(&[px(3, 3)], |d, r| copy_from_master(&master, stride, d, r)));

        let s = published_slot(&rb);
        assert_eq!(token_index(rb.latest.load(Ordering::Acquire)), 2);
        assert_eq!(s.damage_from, 0);
        assert_eq!(s.data, master);
        rb.end_read();
    }

    #[test]
    fn fencepost_range_empty_copies_only_this_frame() {
        let (w, h) = (10u32, 10u32);
        let rb = RenderBuffer::new(w, h);
        let cur_v = 4u64;
        rb.latest.store(token(cur_v, 0), Ordering::Release);
        unsafe {
            (*rb.slots[1].get()).version = cur_v;
        }
        {
            let log = unsafe { &mut *rb.damage_log.get() };
            log.clear();
            log.push_back((cur_v, vec![px(5, 5)]));
        }

        let this = px(4, 4);
        let mut recorded = Vec::new();
        assert!(rb.commit_damage(&[this], |_d, r| recorded.push(r)));
        let s = unsafe { &*rb.slots[1].get() };
        assert_eq!(s.version, cur_v + 1);
        assert_eq!(s.damage_from, cur_v);
        assert_eq!(recorded, vec![this]);
        assert_eq!(s.damage, vec![this.x0, this.y0, this.x1, this.y1]);
    }

    #[test]
    fn fencepost_oldest_at_boundary_uses_partial() {
        let (w, h) = (10u32, 10u32);
        let rb = RenderBuffer::new(w, h);
        let cur_v = 4u64;
        let w_v = 2u64;
        rb.latest.store(token(cur_v, 0), Ordering::Release);
        unsafe {
            (*rb.slots[1].get()).version = w_v;
        }
        let a = px(0, 0);
        let b = px(3, 3);
        {
            let log = unsafe { &mut *rb.damage_log.get() };
            log.clear();
            log.push_back((w_v + 1, vec![a]));
            log.push_back((cur_v, vec![b]));
        }

        let c = px(6, 6);
        let mut recorded = Vec::new();
        assert!(rb.commit_damage(&[c], |_d, r| recorded.push(r)));
        let s = unsafe { &*rb.slots[1].get() };
        assert_eq!(s.damage_from, w_v);
        assert_eq!(recorded, merge_damage(&[a, b, c], full_rect(w, h)));
    }

    #[test]
    fn fencepost_oldest_beyond_range_forces_full() {
        let (w, h) = (10u32, 10u32);
        let rb = RenderBuffer::new(w, h);
        let cur_v = 5u64;
        let w_v = 2u64;
        rb.latest.store(token(cur_v, 0), Ordering::Release);
        unsafe {
            (*rb.slots[1].get()).version = w_v;
        }
        {
            let log = unsafe { &mut *rb.damage_log.get() };
            log.clear();
            log.push_back((w_v + 2, vec![px(3, 3)]));
            log.push_back((cur_v, vec![px(5, 5)]));
        }

        let mut recorded = Vec::new();
        assert!(rb.commit_damage(&[px(6, 6)], |_d, r| recorded.push(r)));
        let s = unsafe { &*rb.slots[1].get() };
        assert_eq!(s.damage_from, 0);
        assert_eq!(recorded, vec![full_rect(w, h)]);
        assert_eq!(s.damage, vec![0, 0, w as i32, h as i32]);
    }

    #[test]
    fn resize_forces_full_reconstruction() {
        let rb = RenderBuffer::new(2, 2);
        assert!(rb.commit_damage(&[full_rect(2, 2)], |d, _r| d.fill(0x11)));
        assert!(rb.commit_damage(&[full_rect(2, 2)], |d, _r| d.fill(0x22)));

        rb.resize(3, 2);
        let mut recorded = Vec::new();
        assert!(rb.commit_damage(&[px(0, 0)], |_d, r| recorded.push(r)));
        let s = published_slot(&rb);
        assert_eq!(s.damage_from, 0);
        assert_eq!(recorded, vec![full_rect(3, 2)]);
        assert_eq!(s.damage, vec![0, 0, 3, 2]);
    }

    #[test]
    fn commit_damage_rejects_reentrant_producer_without_publishing() {
        let rb = RenderBuffer::new(2, 2);
        assert!(rb.commit_damage(&[full_rect(2, 2)], |d, _r| d.fill(0x33)));
        let latest_before = rb.latest.load(Ordering::Acquire);
        let log_len_before = unsafe { (*rb.damage_log.get()).len() };

        rb.writing.store(true, Ordering::Release);
        let mut called = false;
        let prev = std::panic::take_hook();
        std::panic::set_hook(Box::new(|_| {}));
        let res = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            rb.commit_damage(&[full_rect(2, 2)], |_d, _r| {
                called = true;
            })
        }));
        std::panic::set_hook(prev);

        if let Ok(published) = res {
            assert!(!published);
        }
        assert!(!called);
        assert_eq!(rb.latest.load(Ordering::Acquire), latest_before);
        assert_eq!(unsafe { (*rb.damage_log.get()).len() }, log_len_before);
    }

    #[test]
    fn pinned_slot_stays_stable_while_writer_advances() {
        let rb = RenderBuffer::new(2, 2);
        assert!(rb.commit_damage(&[full_rect(2, 2)], |d, _r| d.fill(0x01)));
        assert!(rb.begin_read());
        let pinned_idx = rb.reading.load(Ordering::Acquire);
        let pinned_ver = rb.pinned_version();
        let pinned_ptr = rb.pinned_slot().unwrap().data.as_ptr();

        assert!(rb.commit_damage(&[full_rect(2, 2)], |d, _r| d.fill(0x02)));
        assert!(rb.commit_damage(&[full_rect(2, 2)], |d, _r| d.fill(0x03)));

        assert_eq!(rb.reading.load(Ordering::Acquire), pinned_idx);
        assert_eq!(rb.pinned_version(), pinned_ver);
        assert_eq!(rb.pinned_slot().unwrap().data.as_ptr(), pinned_ptr);
        assert!(token_version(rb.latest.load(Ordering::Acquire)) > pinned_ver);
        rb.end_read();
    }

    #[test]
    fn pinned_accessors_match_pinned_slot() {
        let rb = RenderBuffer::new(4, 4);
        assert!(rb.commit_damage(&[full_rect(4, 4)], |d, _r| d.fill(0x01)));
        assert!(rb.begin_read());
        let slot = published_slot(&rb);
        assert_eq!(rb.pinned_version(), slot.version);
        assert_eq!(rb.pinned_damage_from(), slot.damage_from);
        assert_eq!(rb.pinned_damage(), slot.damage.as_slice());
        assert_eq!(rb.pinned_version(), 1);
        assert_eq!(rb.pinned_damage_from(), 0);
        assert_eq!(rb.pinned_damage(), &[0, 0, 4, 4]);
        rb.end_read();

        assert!(rb.commit_damage(&[px(1, 1)], |_d, _r| {}));
        assert!(rb.commit_damage(&[px(2, 2)], |_d, _r| {}));
        assert!(rb.begin_read());
        let slot = published_slot(&rb);
        assert_eq!(rb.pinned_version(), slot.version);
        assert_eq!(rb.pinned_damage_from(), slot.damage_from);
        assert_eq!(rb.pinned_damage(), slot.damage.as_slice());
        assert_eq!(rb.pinned_version(), 3);
        assert_eq!(rb.pinned_damage_from(), 1);
        rb.end_read();
    }

    #[test]
    fn pinned_accessors_default_when_unpinned() {
        let rb = RenderBuffer::new(2, 2);
        assert_eq!(rb.pinned_version(), 0);
        assert_eq!(rb.pinned_damage_from(), 0);
        assert_eq!(rb.pinned_damage(), &[] as &[i32]);
    }

    fn commit_full(rb: &RenderBuffer, w: u32, h: u32, seed: u8) {
        rb.commit_damage(&[full_rect(w, h)], |data, r| {
            for y in r.y0..r.y1 {
                for x in r.x0..r.x1 {
                    let off = y as usize * w as usize * 4 + x as usize * 4;
                    data[off..off + 4].fill(seed.wrapping_add((x + y * w as i32) as u8));
                }
            }
        });
    }

    #[test]
    fn read_pinned_into_copies_full_range() {
        let rb = RenderBuffer::new(4, 3);
        commit_full(&rb, 4, 3, 7);
        assert!(rb.begin_read());
        let mut dst = vec![0u8; 4 * 3 * 4];
        assert!(unsafe { rb.read_pinned_into(dst.as_mut_ptr(), dst.len(), 0, 3) });
        let slot = rb.pinned_slot().unwrap();
        assert_eq!(&dst[..], &slot.data[..]);
        rb.end_read();
    }

    #[test]
    fn read_pinned_into_partial_rows_leave_rest_untouched() {
        let rb = RenderBuffer::new(4, 3);
        commit_full(&rb, 4, 3, 7);
        assert!(rb.begin_read());
        let mut dst = vec![0xAAu8; 4 * 3 * 4];
        assert!(unsafe { rb.read_pinned_into(dst.as_mut_ptr(), dst.len(), 1, 2) });
        let slot = rb.pinned_slot().unwrap();
        let stride = 4 * 4;
        assert!(dst[..stride].iter().all(|&b| b == 0xAA));
        assert_eq!(&dst[stride..2 * stride], &slot.data[stride..2 * stride]);
        assert!(dst[2 * stride..].iter().all(|&b| b == 0xAA));
        rb.end_read();
    }

    #[test]
    fn read_pinned_into_without_pin_fails() {
        let rb = RenderBuffer::new(4, 3);
        commit_full(&rb, 4, 3, 7);
        let mut dst = vec![0u8; 4 * 3 * 4];
        assert!(!unsafe { rb.read_pinned_into(dst.as_mut_ptr(), dst.len(), 0, 3) });
    }

    #[test]
    fn read_pinned_into_wrong_len_fails() {
        let rb = RenderBuffer::new(4, 3);
        commit_full(&rb, 4, 3, 7);
        assert!(rb.begin_read());
        let mut dst = vec![0u8; 4 * 3 * 4 - 4];
        assert!(!unsafe { rb.read_pinned_into(dst.as_mut_ptr(), dst.len(), 0, 3) });
        rb.end_read();
    }

    #[test]
    fn read_pinned_into_row_range_out_of_bounds_fails() {
        let rb = RenderBuffer::new(4, 3);
        commit_full(&rb, 4, 3, 7);
        assert!(rb.begin_read());
        let mut dst = vec![0u8; 4 * 3 * 4];
        assert!(!unsafe { rb.read_pinned_into(dst.as_mut_ptr(), dst.len(), 0, 4) });
        assert!(!unsafe { rb.read_pinned_into(dst.as_mut_ptr(), dst.len(), 2, 1) });
        rb.end_read();
    }
}
