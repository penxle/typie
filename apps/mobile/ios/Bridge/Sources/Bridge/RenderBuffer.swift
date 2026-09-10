import Foundation

@_silgen_name("render_buffer_allocate")
private func nativeAllocate() -> Int64

@_silgen_name("render_buffer_free")
private func nativeFree(_ handle: Int64) -> Void

@_silgen_name("render_buffer_begin_read")
private func nativeBeginRead(_ handle: Int64) -> Bool

@_silgen_name("render_buffer_end_read")
private func nativeEndRead(_ handle: Int64) -> Void

@_silgen_name("render_buffer_width")
private func nativeGetPixelWidth(_ handle: Int64) -> Int32

@_silgen_name("render_buffer_height")
private func nativeGetPixelHeight(_ handle: Int64) -> Int32

@_silgen_name("render_buffer_pinned_editor_revision")
private func nativeGetPinnedEditorRevision(_ handle: Int64) -> Int64

@_silgen_name("render_buffer_pinned_frame_key")
private func nativeGetPinnedFrameKey(_ handle: Int64) -> Int64

@_silgen_name("render_buffer_pinned_tile_count")
private func nativeGetPinnedTileCount(_ handle: Int64) -> Int32

@_silgen_name("render_buffer_pinned_tile_bounds")
private func nativeGetPinnedTileBounds(_ handle: Int64, _ index: Int32) -> Int64

@_silgen_name("render_buffer_pinned_tile_pixels")
private func nativeGetPinnedTilePixels(_ handle: Int64, _ index: Int32) -> Int64

@_silgen_name("render_buffer_pinned_tile_version")
private func nativeGetPinnedTileVersion(_ handle: Int64, _ index: Int32) -> Int64

@_silgen_name("render_buffer_read_pinned_tile_into")
private func nativeReadPinnedTileInto(_ handle: Int64, _ index: Int32, _ dstAddr: Int64, _ dstLen: Int64) -> Bool

@objc public class RenderBuffer: NSObject {
    @objc public static func allocate() -> Int64 {
        nativeAllocate()
    }

    @objc public static func free(_ handle: Int64) -> Void {
        nativeFree(handle)
    }

    @objc public static func beginRead(_ handle: Int64) -> Bool {
        nativeBeginRead(handle)
    }

    @objc public static func endRead(_ handle: Int64) -> Void {
        nativeEndRead(handle)
    }

    @objc public static func width(_ handle: Int64) -> Int32 {
        nativeGetPixelWidth(handle)
    }

    @objc public static func height(_ handle: Int64) -> Int32 {
        nativeGetPixelHeight(handle)
    }

    @objc public static func pinnedEditorRevision(_ handle: Int64) -> Int64 {
        nativeGetPinnedEditorRevision(handle)
    }

    @objc public static func pinnedFrameKey(_ handle: Int64) -> Int64 {
        nativeGetPinnedFrameKey(handle)
    }

    @objc public static func pinnedTileCount(_ handle: Int64) -> Int32 {
        nativeGetPinnedTileCount(handle)
    }

    @objc public static func pinnedTileBounds(_ handle: Int64, _ index: Int32) -> Int64 {
        nativeGetPinnedTileBounds(handle, index)
    }

    @objc public static func pinnedTilePixels(_ handle: Int64, _ index: Int32) -> Int64 {
        nativeGetPinnedTilePixels(handle, index)
    }

    @objc public static func pinnedTileVersion(_ handle: Int64, _ index: Int32) -> Int64 {
        nativeGetPinnedTileVersion(handle, index)
    }

    @objc public static func readPinnedTileInto(_ handle: Int64, _ index: Int32, _ dstAddr: Int64, _ dstLen: Int64) -> Bool {
        nativeReadPinnedTileInto(handle, index, dstAddr, dstLen)
    }

}
