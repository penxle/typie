import EditorFFI
import Testing

@testable import Editor

@Suite struct RenderBufferTests {
  @Test func rendersAPageThroughTheRenderBuffer() throws {
    let host = try REditorHost(icuData: EditorICU.load())
    let editor = try host.createEditorFromDoc(
      doc: EditorJSON.encode(testDocument([horizontalRuleEntry])),
      viewport: EditorJSON.encode(testViewport))
    let request = try editor.enqueueRequest(messages: [
      EditorJSON.encode(Message.system(event: .initialize))
    ])
    let tick = try EditorJSON.decode(TickResult.self, from: editor.tickThrough(requestId: request))
    let revision = try EditorJSON.encode(tick.revision)

    let buffer = try #require(RenderBuffer.allocate())
    defer { buffer.free() }
    try editor.attachSurface(
      page: 0, handle: UInt64(buffer.handle), width: 794, height: 1123, scaleFactor: 1)
    defer { try? editor.detachSurface(page: 0) }
    try editor.configureSurfaceTiles(
      page: 0,
      bounds: [0, 0, 512, 512, 512, 0, 794, 512, 0, 512, 512, 1024, 512, 512, 794, 1024])
    _ = try editor.renderSurface(page: 0, requestedRevision: revision)

    try #require(buffer.beginRead())
    defer { buffer.endRead() }
    #expect(buffer.pinnedTileCount > 0)
    var inked = false
    for index in 0..<buffer.pinnedTileCount {
      guard let bounds = buffer.pinnedTileBounds(at: index) else { continue }
      let width = Int(bounds.right - bounds.left) + 2
      let height = Int(bounds.bottom - bounds.top) + 2
      var pixels = [UInt8](repeating: 0, count: width * height * 4)
      let copied = pixels.withUnsafeMutableBytes {
        buffer.readPinnedTile(at: index, into: $0.baseAddress!, length: $0.count)
      }
      #expect(copied)
      inked = inked || stride(from: 3, to: pixels.count, by: 4).contains { pixels[$0] > 0 }
    }
    #expect(inked)
  }
}
