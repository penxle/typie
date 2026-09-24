import CoreGraphics
import EditorFFI
import Foundation

@testable import Editor

func frameRequest(scrollY: Double = 0, height: Double = 844, debug: Bool = false)
  -> ViewportRequest
{
  ViewportRequest(
    scrollX: 0, scrollY: scrollY, width: 390, height: height, occlusionTop: 0,
    occlusionBottom: 0, deviceScale: 3, headerHeight: 0, timeMs: 0, debug: debug, destination: nil)
}

func fixtureFamily(_ fixture: FontFixture, named name: String) -> EditorFontFamily {
  EditorFontFamily(name: name, source: .default, fonts: fixture.family.fonts)
}

@MainActor
func preloadFixtureFont(_ resources: EditorResources, family: String, cacheDirectory: URL)
  async throws
{
  let fixture = try FontFixture()
  let network = FakeFontNetwork(body: fixture.read)
  let loader = FontLoader(
    target: resources, fetch: network.fetch, sleep: { _ in }, cacheDirectory: cacheDirectory)
  try loader.apply([fixtureFamily(fixture, named: family)])
  await loader.handle(
    family: family, weight: 400,
    required: [.manifest, .base] + (0..<fixture.chunkCount).map { .chunk(id: UInt16($0)) },
    prefetch: [])
}

func setTiles(_ update: EditorFrameUpdate) -> [(key: EditorTileKey, image: CGImage)] {
  update.tiles.compactMap { change in
    guard case .set(let key, _, let image) = change else { return nil }
    return (key, image)
  }
}

func hasInk(_ image: CGImage) -> Bool {
  guard let data = image.dataProvider?.data as Data? else { return false }
  return stride(from: 3, to: data.count, by: 4).contains { data[$0] != 0 }
}

func testImage() -> CGImage {
  CGContext(
    data: nil, width: 4, height: 4, bitsPerComponent: 8, bytesPerRow: 16,
    space: CGColorSpace(name: CGColorSpace.sRGB)!,
    bitmapInfo: CGImageAlphaInfo.premultipliedLast.rawValue
  )!.makeImage()!
}

func testPage(_ index: UInt32, y: Double, height: Double = 1000) -> FramePage {
  FramePage(index: index, x: 0, y: y, width: 390, height: height)
}

func testUpdate(
  pages: [FramePage], tiles: [EditorTileChange] = [], zoom: Double = 1, deviceScale: Double = 3,
  contentHeight: Double = 5000
) -> EditorFrameUpdate {
  EditorFrameUpdate(
    geometry: FrameGeometry(
      id: 1, revision: Revision(value: 1), tick: nil, layout: .continuous, zoom: zoom,
      rasterScale: zoom * deviceScale, contentWidth: 390, contentHeight: contentHeight,
      pageCount: UInt32(pages.count),
      body: FrameBody(
        pagesTop: 40, topSpacer: 40, pagesBottom: contentHeight, bottomPadding: 0,
        minimumBodyBottom: 844),
      pages: pages, tiles: [], position: nil, debug: nil, needsNextFrame: false,
      fillRemaining: false),
    tiles: tiles)
}

func tileKey(_ page: Int, _ x0: Int32, _ y0: Int32, _ x1: Int32, _ y1: Int32) -> EditorTileKey {
  EditorTileKey(page: page, bounds: FramePxRect(x0: x0, y0: y0, x1: x1, y1: y1))
}

let parkedSleep: @Sendable (Duration) async throws -> Void = { _ in
  try await Task.sleep(for: .seconds(3600))
}

@MainActor
final class TestClock {
  var now = ContinuousClock.now

  func advance(_ duration: Duration) {
    now += duration
  }
}
