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
}
