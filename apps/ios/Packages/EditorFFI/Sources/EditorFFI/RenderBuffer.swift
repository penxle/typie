import Foundation

@_silgen_name("render_buffer_allocate")
private func nativeAllocate() -> Int64

@_silgen_name("render_buffer_free")
private func nativeFree(_ handle: Int64)

@_silgen_name("render_buffer_begin_read")
private func nativeBeginRead(_ handle: Int64) -> Bool

@_silgen_name("render_buffer_end_read")
private func nativeEndRead(_ handle: Int64)

@_silgen_name("render_buffer_width")
private func nativePixelWidth(_ handle: Int64) -> Int32

@_silgen_name("render_buffer_height")
private func nativePixelHeight(_ handle: Int64) -> Int32

@_silgen_name("render_buffer_pinned_editor_revision")
private func nativePinnedEditorRevision(_ handle: Int64) -> Int64

@_silgen_name("render_buffer_pinned_frame_key")
private func nativePinnedFrameKey(_ handle: Int64) -> Int64

@_silgen_name("render_buffer_pinned_tile_count")
private func nativePinnedTileCount(_ handle: Int64) -> Int32

@_silgen_name("render_buffer_pinned_tile_bounds")
private func nativePinnedTileBounds(_ handle: Int64, _ index: Int32) -> Int64

@_silgen_name("render_buffer_pinned_tile_pixels")
private func nativePinnedTilePixels(_ handle: Int64, _ index: Int32) -> Int64

@_silgen_name("render_buffer_pinned_tile_version")
private func nativePinnedTileVersion(_ handle: Int64, _ index: Int32) -> Int64

@_silgen_name("render_buffer_read_pinned_tile_into")
private func nativeReadPinnedTileInto(
  _ handle: Int64, _ index: Int32, _ destination: Int64, _ length: Int64
) -> Bool

public struct RenderTileBounds: Sendable, Equatable {
  public let left: Int32
  public let top: Int32
  public let right: Int32
  public let bottom: Int32
}

public struct RenderBuffer: @unchecked Sendable {
  public let handle: Int64

  public static func allocate() -> RenderBuffer? {
    let handle = nativeAllocate()
    return handle == 0 ? nil : RenderBuffer(handle: handle)
  }

  public func free() {
    nativeFree(handle)
  }

  public func beginRead() -> Bool {
    nativeBeginRead(handle)
  }

  public func endRead() {
    nativeEndRead(handle)
  }

  public var pixelWidth: Int32 { nativePixelWidth(handle) }
  public var pixelHeight: Int32 { nativePixelHeight(handle) }
  public var pinnedEditorRevision: Int64 { nativePinnedEditorRevision(handle) }
  public var pinnedFrameKey: Int64 { nativePinnedFrameKey(handle) }
  public var pinnedTileCount: Int32 { nativePinnedTileCount(handle) }

  public func pinnedTileBounds(at index: Int32) -> RenderTileBounds? {
    guard
      let pointer = UnsafePointer<Int32>(bitPattern: Int(nativePinnedTileBounds(handle, index)))
    else { return nil }
    return RenderTileBounds(
      left: pointer[0], top: pointer[1], right: pointer[2], bottom: pointer[3])
  }

  public func pinnedTilePixels(at index: Int32) -> UnsafeRawPointer? {
    UnsafeRawPointer(bitPattern: Int(nativePinnedTilePixels(handle, index)))
  }

  public func pinnedTileVersion(at index: Int32) -> Int64 {
    nativePinnedTileVersion(handle, index)
  }

  public func readPinnedTile(
    at index: Int32, into destination: UnsafeMutableRawPointer, length: Int
  ) -> Bool {
    nativeReadPinnedTileInto(handle, index, Int64(Int(bitPattern: destination)), Int64(length))
  }
}
