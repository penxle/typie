import Foundation

@_silgen_name("render_buffer_allocate")
private func renderBufferAllocate(_ width: Int32, _ height: Int32) -> Int64

@_silgen_name("render_buffer_free")
private func renderBufferFree(_ handle: Int64)

@_silgen_name("render_buffer_resize")
private func renderBufferResize(_ handle: Int64, _ width: Int32, _ height: Int32)

@_silgen_name("render_buffer_begin_read")
private func renderBufferBeginRead(_ handle: Int64) -> Bool

@_silgen_name("render_buffer_end_read")
private func renderBufferEndRead(_ handle: Int64)

@_silgen_name("render_buffer_data_pointer")
private func renderBufferDataPointer(_ handle: Int64) -> Int64

@_silgen_name("render_buffer_width")
private func renderBufferWidth(_ handle: Int64) -> Int32

@_silgen_name("render_buffer_height")
private func renderBufferHeight(_ handle: Int64) -> Int32

@_silgen_name("render_buffer_pinned_version")
private func renderBufferPinnedVersion(_ handle: Int64) -> Int64

@_silgen_name("render_buffer_pinned_damage_from")
private func renderBufferPinnedDamageFrom(_ handle: Int64) -> Int64

@_silgen_name("render_buffer_pinned_damage_pointer")
private func renderBufferPinnedDamagePointer(_ handle: Int64) -> Int64

@_silgen_name("render_buffer_pinned_damage_count")
private func renderBufferPinnedDamageCount(_ handle: Int64) -> Int32

@_silgen_name("render_buffer_read_pinned_into")
private func renderBufferReadPinnedInto(_ handle: Int64, _ dst: Int64, _ dstLen: Int64, _ rowFrom: Int32, _ rowTo: Int32) -> Bool

@objc public class RenderBuffer: NSObject {
    @objc public static func allocate(_ width: Int32, _ height: Int32) -> Int64 {
        renderBufferAllocate(width, height)
    }

    @objc public static func free(_ handle: Int64) {
        renderBufferFree(handle)
    }

    @objc public static func resize(_ handle: Int64, _ width: Int32, _ height: Int32) {
        renderBufferResize(handle, width, height)
    }

    @objc public static func beginRead(_ handle: Int64) -> Bool {
        renderBufferBeginRead(handle)
    }

    @objc public static func endRead(_ handle: Int64) {
        renderBufferEndRead(handle)
    }

    @objc public static func dataPointer(_ handle: Int64) -> Int64 {
        renderBufferDataPointer(handle)
    }

    @objc public static func width(_ handle: Int64) -> Int32 {
        renderBufferWidth(handle)
    }

    @objc public static func height(_ handle: Int64) -> Int32 {
        renderBufferHeight(handle)
    }

    @objc public static func pinnedVersion(_ handle: Int64) -> Int64 {
        renderBufferPinnedVersion(handle)
    }

    @objc public static func pinnedDamageFrom(_ handle: Int64) -> Int64 {
        renderBufferPinnedDamageFrom(handle)
    }

    @objc public static func pinnedDamagePointer(_ handle: Int64) -> Int64 {
        renderBufferPinnedDamagePointer(handle)
    }

    @objc public static func pinnedDamageCount(_ handle: Int64) -> Int32 {
        renderBufferPinnedDamageCount(handle)
    }

    @objc public static func readPinnedInto(_ handle: Int64, _ dst: Int64, _ dstLen: Int64, _ rowFrom: Int32, _ rowTo: Int32) -> Bool {
        renderBufferReadPinnedInto(handle, dst, dstLen, rowFrom, rowTo)
    }
}
