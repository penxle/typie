import CoreGraphics
internal import EditorFFI
import Foundation

struct EditorTileKey: Hashable {
  var page: Int
  var bounds: FramePxRect
}

enum EditorTileChange {
  case set(EditorTileKey, version: UInt64, image: CGImage)
  case drop(EditorTileKey)
}

struct EditorFrameUpdate {
  var geometry: FrameGeometry
  var tiles: [EditorTileChange]
}

enum EditorFrameError: Error, Equatable {
  case missingPixels(UInt32)
  case invalidTile(FramePxRect)
}

@MainActor
final class EditorFrameSource {
  private static let colorSpace = CGColorSpace(name: CGColorSpace.sRGB)!

  private let viewport: REditorViewport

  init(engine: EditorEngine) throws {
    viewport = try REditorViewport(editor: engine.raw)
  }

  func frame(_ request: ViewportRequest) throws -> EditorFrameUpdate {
    try Self.update(from: viewport.frame(request: EditorJSON.encode(request)))
  }

  func fill(budgetMs: Double) throws -> EditorFrameUpdate? {
    try viewport.fill(budgetMs: budgetMs).map(Self.update(from:))
  }

  func presented(_ id: UInt64) throws {
    try viewport.presented(frameId: id)
  }

  static func update(from frame: REditorFrame) throws -> EditorFrameUpdate {
    try withExtendedLifetime(frame) {
      let geometry = try EditorJSON.decode(FrameGeometry.self, from: frame.geometry())
      let count = try frame.pixelCount()
      let tiles = try geometry.tiles.map { tile -> EditorTileChange in
        switch tile {
        case .set(let page, let bounds, let version, let pixels):
          guard pixels < count else { throw EditorFrameError.missingPixels(pixels) }
          let image = try image(
            address: frame.pixelAddress(index: pixels), length: frame.pixelLength(index: pixels),
            bounds: bounds)
          return .set(
            EditorTileKey(page: Int(page), bounds: bounds), version: version, image: image)
        case .drop(let page, let bounds):
          return .drop(EditorTileKey(page: Int(page), bounds: bounds))
        }
      }
      return EditorFrameUpdate(geometry: geometry, tiles: tiles)
    }
  }

  static func image(address: UInt64, length: UInt64, bounds: FramePxRect) throws -> CGImage {
    let width = Int(bounds.x1) - Int(bounds.x0) + 2
    let height = Int(bounds.y1) - Int(bounds.y0) + 2
    let bytesPerRow = width * 4
    guard width > 2, height > 2, length == UInt64(bytesPerRow * height),
      let pixels = UnsafeRawPointer(bitPattern: UInt(address)),
      let data = CFDataCreate(
        kCFAllocatorDefault, pixels.assumingMemoryBound(to: UInt8.self), bytesPerRow * height),
      let provider = CGDataProvider(data: data),
      let image = CGImage(
        width: width, height: height, bitsPerComponent: 8, bitsPerPixel: 32,
        bytesPerRow: bytesPerRow, space: colorSpace,
        bitmapInfo: CGBitmapInfo(rawValue: CGImageAlphaInfo.premultipliedLast.rawValue),
        provider: provider, decode: nil, shouldInterpolate: false, intent: .defaultIntent)
    else { throw EditorFrameError.invalidTile(bounds) }
    return image
  }
}
