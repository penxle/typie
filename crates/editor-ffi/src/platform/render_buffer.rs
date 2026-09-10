use std::sync::{Arc, Mutex};

use super::tiled_surface::RenderedTile;

pub struct RenderedFrame {
    pub width: u32,
    pub height: u32,
    pub editor_revision: u64,
    pub frame_key: u64,
    pub tiles: Vec<RenderedTile>,
}

#[derive(Default)]
struct BufferState {
    latest: Option<Arc<RenderedFrame>>,
    reading: Option<Arc<RenderedFrame>>,
    dirty: bool,
}

/// One producer publishes immutable frames; one reader pins a frame while copying new tiles.
/// Publishing a replacement cannot invalidate pointers into the pinned frame.
#[derive(Default)]
pub struct RenderBuffer {
    state: Mutex<BufferState>,
}

impl RenderBuffer {
    pub fn publish(&self, frame: RenderedFrame) -> bool {
        let Ok(mut state) = self.state.lock() else {
            return false;
        };
        state.latest = Some(Arc::new(frame));
        state.dirty = true;
        true
    }

    pub fn begin_read(&self) -> bool {
        let Ok(mut state) = self.state.lock() else {
            return false;
        };
        if !state.dirty || state.reading.is_some() {
            return false;
        }
        state.reading = state.latest.clone();
        state.dirty = false;
        state.reading.is_some()
    }

    pub fn end_read(&self) {
        if let Ok(mut state) = self.state.lock() {
            state.reading = None;
        }
    }

    fn pinned<T: Default>(&self, read: impl FnOnce(&RenderedFrame) -> T) -> T {
        self.state
            .lock()
            .ok()
            .and_then(|s| s.reading.as_deref().map(read))
            .unwrap_or_default()
    }

    fn tile<T: Default>(&self, index: i32, read: impl FnOnce(&RenderedTile) -> T) -> T {
        self.pinned(|frame| {
            frame
                .tiles
                .get(index as usize)
                .map(read)
                .unwrap_or_default()
        })
    }

    /// # Safety
    /// dst must point to dst_len writable bytes and must not overlap the pinned tile.
    unsafe fn read_tile_into(&self, index: i32, dst: *mut u8, dst_len: usize) -> bool {
        self.tile(index, |tile| {
            if tile.pixels.len() != dst_len {
                return false;
            }
            unsafe {
                std::ptr::copy_nonoverlapping(tile.pixels.as_ptr(), dst, dst_len);
            }
            true
        })
    }
}

// All pixel and bounds pointers below are valid only between beginRead and endRead.

#[cfg(target_os = "ios")]
#[unsafe(no_mangle)]
pub extern "C" fn render_buffer_allocate() -> i64 {
    Box::into_raw(Box::new(RenderBuffer::default())) as i64
}

#[cfg(not(target_os = "ios"))]
#[unsafe(no_mangle)]
pub extern "C" fn Java_co_typie_editor_render_RenderBuffer_allocate(
    _env: *mut std::ffi::c_void,
    _class: *mut std::ffi::c_void,
) -> i64 {
    Box::into_raw(Box::new(RenderBuffer::default())) as i64
}

#[cfg(target_os = "ios")]
#[unsafe(no_mangle)]
pub extern "C" fn render_buffer_free(handle: i64) {
    if handle != 0 {
        unsafe {
            drop(Box::from_raw(handle as *mut RenderBuffer));
        }
    }
}

#[cfg(not(target_os = "ios"))]
#[unsafe(no_mangle)]
pub extern "C" fn Java_co_typie_editor_render_RenderBuffer_free(
    _env: *mut std::ffi::c_void,
    _class: *mut std::ffi::c_void,
    handle: i64,
) {
    if handle != 0 {
        unsafe {
            drop(Box::from_raw(handle as *mut RenderBuffer));
        }
    }
}

#[cfg(target_os = "ios")]
#[unsafe(no_mangle)]
pub extern "C" fn render_buffer_begin_read(handle: i64) -> bool {
    if handle == 0 {
        return false;
    }
    unsafe { (&*(handle as *const RenderBuffer)).begin_read() }
}

#[cfg(not(target_os = "ios"))]
#[unsafe(no_mangle)]
pub extern "C" fn Java_co_typie_editor_render_RenderBuffer_beginRead(
    _env: *mut std::ffi::c_void,
    _class: *mut std::ffi::c_void,
    handle: i64,
) -> bool {
    if handle == 0 {
        return false;
    }
    unsafe { (&*(handle as *const RenderBuffer)).begin_read() }
}

#[cfg(target_os = "ios")]
#[unsafe(no_mangle)]
pub extern "C" fn render_buffer_end_read(handle: i64) {
    if handle != 0 {
        unsafe {
            (&*(handle as *const RenderBuffer)).end_read();
        }
    }
}

#[cfg(not(target_os = "ios"))]
#[unsafe(no_mangle)]
pub extern "C" fn Java_co_typie_editor_render_RenderBuffer_endRead(
    _env: *mut std::ffi::c_void,
    _class: *mut std::ffi::c_void,
    handle: i64,
) {
    if handle != 0 {
        unsafe {
            (&*(handle as *const RenderBuffer)).end_read();
        }
    }
}

#[cfg(target_os = "ios")]
#[unsafe(no_mangle)]
pub extern "C" fn render_buffer_width(handle: i64) -> i32 {
    if handle == 0 {
        return 0;
    }
    let b = unsafe { &*(handle as *const RenderBuffer) };
    b.pinned(|f| f.width as i32)
}

#[cfg(not(target_os = "ios"))]
#[unsafe(no_mangle)]
pub extern "C" fn Java_co_typie_editor_render_RenderBuffer_getPixelWidth(
    _env: *mut std::ffi::c_void,
    _class: *mut std::ffi::c_void,
    handle: i64,
) -> i32 {
    if handle == 0 {
        return 0;
    }
    let b = unsafe { &*(handle as *const RenderBuffer) };
    b.pinned(|f| f.width as i32)
}

#[cfg(target_os = "ios")]
#[unsafe(no_mangle)]
pub extern "C" fn render_buffer_height(handle: i64) -> i32 {
    if handle == 0 {
        return 0;
    }
    let b = unsafe { &*(handle as *const RenderBuffer) };
    b.pinned(|f| f.height as i32)
}

#[cfg(not(target_os = "ios"))]
#[unsafe(no_mangle)]
pub extern "C" fn Java_co_typie_editor_render_RenderBuffer_getPixelHeight(
    _env: *mut std::ffi::c_void,
    _class: *mut std::ffi::c_void,
    handle: i64,
) -> i32 {
    if handle == 0 {
        return 0;
    }
    let b = unsafe { &*(handle as *const RenderBuffer) };
    b.pinned(|f| f.height as i32)
}

#[cfg(target_os = "ios")]
#[unsafe(no_mangle)]
pub extern "C" fn render_buffer_pinned_editor_revision(handle: i64) -> i64 {
    if handle == 0 {
        return 0;
    }
    let b = unsafe { &*(handle as *const RenderBuffer) };
    b.pinned(|f| f.editor_revision as i64)
}

#[cfg(not(target_os = "ios"))]
#[unsafe(no_mangle)]
pub extern "C" fn Java_co_typie_editor_render_RenderBuffer_getPinnedEditorRevision(
    _env: *mut std::ffi::c_void,
    _class: *mut std::ffi::c_void,
    handle: i64,
) -> i64 {
    if handle == 0 {
        return 0;
    }
    let b = unsafe { &*(handle as *const RenderBuffer) };
    b.pinned(|f| f.editor_revision as i64)
}

#[cfg(target_os = "ios")]
#[unsafe(no_mangle)]
pub extern "C" fn render_buffer_pinned_frame_key(handle: i64) -> i64 {
    if handle == 0 {
        return 0;
    }
    let b = unsafe { &*(handle as *const RenderBuffer) };
    b.pinned(|f| f.frame_key as i64)
}

#[cfg(not(target_os = "ios"))]
#[unsafe(no_mangle)]
pub extern "C" fn Java_co_typie_editor_render_RenderBuffer_getPinnedFrameKey(
    _env: *mut std::ffi::c_void,
    _class: *mut std::ffi::c_void,
    handle: i64,
) -> i64 {
    if handle == 0 {
        return 0;
    }
    let b = unsafe { &*(handle as *const RenderBuffer) };
    b.pinned(|f| f.frame_key as i64)
}

#[cfg(target_os = "ios")]
#[unsafe(no_mangle)]
pub extern "C" fn render_buffer_pinned_tile_count(handle: i64) -> i32 {
    if handle == 0 {
        return 0;
    }
    let b = unsafe { &*(handle as *const RenderBuffer) };
    b.pinned(|f| f.tiles.len() as i32)
}

#[cfg(not(target_os = "ios"))]
#[unsafe(no_mangle)]
pub extern "C" fn Java_co_typie_editor_render_RenderBuffer_getPinnedTileCount(
    _env: *mut std::ffi::c_void,
    _class: *mut std::ffi::c_void,
    handle: i64,
) -> i32 {
    if handle == 0 {
        return 0;
    }
    let b = unsafe { &*(handle as *const RenderBuffer) };
    b.pinned(|f| f.tiles.len() as i32)
}

#[cfg(target_os = "ios")]
#[unsafe(no_mangle)]
pub extern "C" fn render_buffer_pinned_tile_bounds(handle: i64, index: i32) -> i64 {
    if handle == 0 {
        return 0;
    }
    let b = unsafe { &*(handle as *const RenderBuffer) };
    b.tile(index, |t| t.bounds.as_ptr() as i64)
}

#[cfg(not(target_os = "ios"))]
#[unsafe(no_mangle)]
pub extern "C" fn Java_co_typie_editor_render_RenderBuffer_getPinnedTileBounds(
    _env: *mut std::ffi::c_void,
    _class: *mut std::ffi::c_void,
    handle: i64,
    index: i32,
) -> i64 {
    if handle == 0 {
        return 0;
    }
    let b = unsafe { &*(handle as *const RenderBuffer) };
    b.tile(index, |t| t.bounds.as_ptr() as i64)
}

#[cfg(target_os = "ios")]
#[unsafe(no_mangle)]
pub extern "C" fn render_buffer_pinned_tile_pixels(handle: i64, index: i32) -> i64 {
    if handle == 0 {
        return 0;
    }
    let b = unsafe { &*(handle as *const RenderBuffer) };
    b.tile(index, |t| t.pixels.as_ptr() as i64)
}

#[cfg(not(target_os = "ios"))]
#[unsafe(no_mangle)]
pub extern "C" fn Java_co_typie_editor_render_RenderBuffer_getPinnedTilePixels(
    _env: *mut std::ffi::c_void,
    _class: *mut std::ffi::c_void,
    handle: i64,
    index: i32,
) -> i64 {
    if handle == 0 {
        return 0;
    }
    let b = unsafe { &*(handle as *const RenderBuffer) };
    b.tile(index, |t| t.pixels.as_ptr() as i64)
}

#[cfg(target_os = "ios")]
#[unsafe(no_mangle)]
pub extern "C" fn render_buffer_pinned_tile_version(handle: i64, index: i32) -> i64 {
    if handle == 0 {
        return 0;
    }
    let b = unsafe { &*(handle as *const RenderBuffer) };
    b.tile(index, |t| t.version as i64)
}

#[cfg(not(target_os = "ios"))]
#[unsafe(no_mangle)]
pub extern "C" fn Java_co_typie_editor_render_RenderBuffer_getPinnedTileVersion(
    _env: *mut std::ffi::c_void,
    _class: *mut std::ffi::c_void,
    handle: i64,
    index: i32,
) -> i64 {
    if handle == 0 {
        return 0;
    }
    let b = unsafe { &*(handle as *const RenderBuffer) };
    b.tile(index, |t| t.version as i64)
}

#[cfg(target_os = "ios")]
#[unsafe(no_mangle)]
pub extern "C" fn render_buffer_read_pinned_tile_into(
    handle: i64,
    index: i32,
    dst: i64,
    dst_len: i64,
) -> bool {
    if handle == 0 || dst == 0 || dst_len < 0 {
        return false;
    }
    unsafe {
        (&*(handle as *const RenderBuffer)).read_tile_into(index, dst as *mut u8, dst_len as usize)
    }
}

#[cfg(not(target_os = "ios"))]
#[unsafe(no_mangle)]
pub extern "C" fn Java_co_typie_editor_render_RenderBuffer_readPinnedTileInto(
    _env: *mut std::ffi::c_void,
    _class: *mut std::ffi::c_void,
    handle: i64,
    index: i32,
    dst: i64,
    dst_len: i64,
) -> bool {
    if handle == 0 || dst == 0 || dst_len < 0 {
        return false;
    }
    unsafe {
        (&*(handle as *const RenderBuffer)).read_tile_into(index, dst as *mut u8, dst_len as usize)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn frame(version: u64, pixels: Arc<[u8]>) -> RenderedFrame {
        RenderedFrame {
            width: 100_000,
            height: 200_000,
            editor_revision: version,
            frame_key: version,
            tiles: vec![RenderedTile {
                bounds: [512, 1024, 513, 1025],
                pixels,
                version,
            }],
        }
    }

    #[test]
    fn reused_pixels_can_be_delivered_to_a_new_consumer_at_the_requested_revision() {
        use super::super::cpu_surface::SurfaceHandle;
        use crate::editor::FrameKey;
        let buffer = RenderBuffer::default();
        let handle = (&buffer as *const RenderBuffer) as u64;
        let mut surface = SurfaceHandle::new(handle, 10_000.0, 20_000.0, 2.0).unwrap();
        assert_eq!(surface.scale_factor(), 2.0);
        assert!(!surface.resize(10_000.0, 20_000.0, 2.0));
        surface.configure_tiles(&[0, 0, 512, 512]).unwrap();
        assert!(surface.needs_render());
        let key = FrameKey { value: 1 };
        assert!(surface.apply_damage(&Default::default(), &[], 1, key));
        assert!(buffer.begin_read());
        let pixels = buffer.tile(0, |tile| tile.pixels.clone());
        buffer.end_read();
        assert!(surface.publish_frame(2, key));
        assert!(buffer.begin_read());
        assert_eq!(
            buffer.pinned(|frame| (frame.editor_revision, frame.frame_key)),
            (2, 1)
        );
        assert!(buffer.tile(0, |tile| Arc::ptr_eq(&tile.pixels, &pixels)));
        buffer.end_read();
    }

    #[test]
    fn pinned_tiles_survive_replacement_and_are_released_after_read() {
        let buffer = RenderBuffer::default();
        let pixels: Arc<[u8]> = Arc::from([1, 2, 3, 4]);
        let weak = Arc::downgrade(&pixels);
        assert!(buffer.publish(frame(1, pixels)));
        assert!(buffer.begin_read());
        let pointer = buffer.tile(0, |t| t.pixels.as_ptr() as usize);
        assert!(buffer.publish(frame(2, Arc::from([5, 6, 7, 8]))));
        assert!(!buffer.begin_read());
        assert_eq!(buffer.pinned(|f| f.frame_key), 1);
        assert_eq!(
            unsafe { std::slice::from_raw_parts(pointer as *const u8, 4) },
            [1, 2, 3, 4]
        );
        assert!(weak.upgrade().is_some());
        buffer.end_read();
        assert!(weak.upgrade().is_none());
        assert!(buffer.begin_read());
        assert_eq!(buffer.pinned(|f| f.frame_key), 2);
        buffer.end_read();
        assert!(!buffer.begin_read());
    }

    #[test]
    fn concurrent_publication_keeps_one_coherent_pinned_frame() {
        let buffer = Arc::new(RenderBuffer::default());
        let writer_buffer = buffer.clone();
        let writer = std::thread::spawn(move || {
            for version in 1..=2000 {
                writer_buffer.publish(frame(version, Arc::from((version as u32).to_ne_bytes())));
            }
        });
        while !writer.is_finished() {
            if buffer.begin_read() {
                let version = buffer.pinned(|f| f.frame_key);
                let mut bytes = [0u8; 4];
                assert!(unsafe { buffer.read_tile_into(0, bytes.as_mut_ptr(), 4) });
                assert_eq!(u32::from_ne_bytes(bytes) as u64, version);
                buffer.end_read();
            }
        }
        writer.join().unwrap();
    }
}
