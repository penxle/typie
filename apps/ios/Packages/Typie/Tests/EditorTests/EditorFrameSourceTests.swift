import CoreGraphics
import EditorFFI
import Foundation
import Testing

@testable import Editor

@MainActor @Suite final class EditorFrameSourceTests {
  private let cacheDirectory = FileManager.default.temporaryDirectory.appending(
    path: "frame-source-\(UUID().uuidString)")

  deinit {
    try? FileManager.default.removeItem(at: cacheDirectory)
  }

  private func engine() async throws -> EditorEngine {
    let resources = try await EditorResources.load()
    try await preloadFixtureFont(resources, family: "Pretendard", cacheDirectory: cacheDirectory)
    let paragraphs = (0..<400).map { testParagraph("Paragraph \($0) with a few plain words.") }
    return try await EditorEngine.make(
      resources: resources, document: fontDocument("Pretendard", paragraphs),
      viewport: testViewport
    ).engine
  }

  @Test func tilePixelsAreCopiedIntoPremultipliedRGBAImagesWithTheirGutter() async throws {
    let engine = try await engine()
    let viewport = try REditorViewport(editor: engine.raw)
    let frame = try viewport.frame(request: EditorJSON.encode(frameRequest()))
    let update = try EditorFrameSource.update(from: frame)

    #expect(update.geometry.pages.map(\.index).contains(0))
    #expect(update.tiles.count == update.geometry.tiles.count)
    var copied = 0
    for (tile, change) in zip(update.geometry.tiles, update.tiles) {
      guard case .set(let page, let bounds, _, let pixels) = tile,
        case .set(let key, _, let image) = change
      else { continue }
      let width = Int(bounds.x1 - bounds.x0) + 2
      let height = Int(bounds.y1 - bounds.y0) + 2
      #expect(key == EditorTileKey(page: Int(page), bounds: bounds))
      #expect(image.width == width)
      #expect(image.height == height)
      #expect(image.bitsPerComponent == 8)
      #expect(image.bitsPerPixel == 32)
      #expect(image.bytesPerRow == width * 4)
      #expect(image.alphaInfo == .premultipliedLast)
      #expect(image.byteOrderInfo == .orderDefault)
      #expect(image.colorSpace?.name == CGColorSpace.sRGB)
      let address = try frame.pixelAddress(index: pixels)
      let pointer = try #require(UnsafeRawPointer(bitPattern: UInt(address)))
      let expected = Data(bytes: pointer, count: Int(try frame.pixelLength(index: pixels)))
      #expect(image.dataProvider?.data as Data? == expected)
      copied += 1
    }
    #expect(copied > 0)
    #expect(setTiles(update).contains { hasInk($0.image) })
    withExtendedLifetime(frame) {}
  }

  @Test func marginTilesArriveThroughFillUntilNothingRemains() async throws {
    let source = try EditorFrameSource(engine: try await engine())
    let first = try source.frame(frameRequest())
    try source.presented(first.geometry.id)
    #expect(first.geometry.fillRemaining)

    var filled = 0
    var remaining = first.geometry.fillRemaining
    while remaining, let update = try source.fill(budgetMs: .infinity) {
      filled += setTiles(update).count
      remaining = update.geometry.fillRemaining
      try source.presented(update.geometry.id)
    }

    #expect(filled > 0)
    #expect(!remaining)
    #expect(try source.fill(budgetMs: 4) == nil)
  }

  @Test func scrollingKeepsTheRevisionAndDropsTilesLeftBehind() async throws {
    let source = try EditorFrameSource(engine: try await engine())
    let first = try source.frame(frameRequest())
    let shown = Set(setTiles(first).map(\.key))
    let far = try source.frame(frameRequest(scrollY: 6000))

    #expect(!shown.isEmpty)
    #expect(far.geometry.revision == first.geometry.revision)
    #expect(setTiles(far).isEmpty)
    let dropped = far.tiles.compactMap { change -> EditorTileKey? in
      guard case .drop(let key) = change else { return nil }
      return key
    }
    #expect(Set(dropped) == shown)
  }

  @Test func tileMemoryThatDoesNotMatchItsBoundsIsRejected() {
    let bounds = FramePxRect(x0: 0, y0: 0, x1: 4, y1: 4)
    var bytes = [UInt8](repeating: 0, count: 6 * 6 * 4)
    bytes.withUnsafeMutableBytes { buffer in
      let address = UInt64(UInt(bitPattern: buffer.baseAddress))
      #expect(throws: EditorFrameError.invalidTile(bounds)) {
        try EditorFrameSource.image(address: address, length: 6 * 6 * 4 - 1, bounds: bounds)
      }
      #expect(throws: EditorFrameError.invalidTile(bounds)) {
        try EditorFrameSource.image(address: 0, length: 6 * 6 * 4, bounds: bounds)
      }
      #expect(
        (try? EditorFrameSource.image(address: address, length: 6 * 6 * 4, bounds: bounds)) != nil)
    }
  }

  @Test func framesWhoseTilesCannotBeCopiedAreRejected() throws {
    let bounds = FramePxRect(x0: 0, y0: 0, x1: 4, y1: 4)
    var geometry = testUpdate(pages: [testPage(0, y: 0)]).geometry
    let frame = StubFrame(noHandle: .init())
    frame.count = 1
    frame.length = 6 * 6 * 4

    geometry.tiles = [.set(page: 0, bounds: bounds, version: 1, pixels: 1)]
    frame.json = try EditorJSON.encode(geometry)
    #expect(throws: EditorFrameError.missingPixels(1)) {
      try EditorFrameSource.update(from: frame)
    }

    geometry.tiles = [
      .drop(page: 0, bounds: FramePxRect(x0: 0, y0: 4, x1: 4, y1: 8)),
      .set(page: 0, bounds: bounds, version: 1, pixels: 0),
    ]
    frame.json = try EditorJSON.encode(geometry)
    #expect(throws: EditorFrameError.invalidTile(bounds)) {
      try EditorFrameSource.update(from: frame)
    }
  }
}

private final class StubFrame: REditorFrame, @unchecked Sendable {
  var json = ""
  var address: UInt64 = 0
  var count: UInt32 = 0
  var length: UInt64 = 0

  override func geometry() throws -> String { json }
  override func pixelAddress(index: UInt32) throws -> UInt64 { address }
  override func pixelCount() throws -> UInt32 { count }
  override func pixelLength(index: UInt32) throws -> UInt64 { length }
}
