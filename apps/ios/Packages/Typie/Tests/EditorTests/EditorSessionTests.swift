import EditorFFI
import Foundation
import Testing

@testable import Editor

@MainActor @Suite final class EditorSessionTests {
  private let cacheDirectory = FileManager.default.temporaryDirectory.appending(
    path: "session-\(UUID().uuidString)")

  deinit {
    try? FileManager.default.removeItem(at: cacheDirectory)
  }

  private func document() -> EditorDocument {
    EditorDocument(plain: fontDocument("Pretendard", [testParagraph("hello")]))
  }

  @Test func initialAndFrameTicksReachTheFontLoader() async throws {
    let fixture = try FontFixture()
    let network = FakeFontNetwork(body: fixture.read)
    let resources = try await EditorResources.load()
    let fonts = FontLoader(
      target: resources, fetch: network.fetch, sleep: { _ in }, cacheDirectory: cacheDirectory)
    try fonts.apply([fixtureFamily(fixture, named: "Pretendard")])
    let session = try await EditorSession.open(
      document(), resources: resources, fonts: fonts, viewport: testViewport)

    await fonts.settle()
    #expect(network.requests == [fixture.url("manifest.v2")])

    var update = try session.frame(frameRequest())
    await fonts.settle()
    #expect(network.requests.contains(fixture.url("base")))

    var inked = false
    var rounds = 0
    while update.geometry.tick != nil, rounds < 16 {
      update = try session.frame(frameRequest())
      inked = inked || setTiles(update).contains { hasInk($0.image) }
      await fonts.settle()
      rounds += 1
    }
    #expect(update.geometry.tick == nil)
    #expect(inked)
    #expect(
      Set(network.requests.filter { $0.path().contains("/chunks/") })
        == Set((0..<fixture.chunkCount).map { fixture.url("chunks/\($0)") }))
  }

  @Test func aFontListAppliedAfterOpeningIsRequestedOnTheNextFrame() async throws {
    let fixture = try FontFixture()
    let network = FakeFontNetwork(body: fixture.read)
    let resources = try await EditorResources.load()
    let fonts = FontLoader(
      target: resources, fetch: network.fetch, sleep: { _ in }, cacheDirectory: cacheDirectory)
    let session = try await EditorSession.open(
      document(), resources: resources, fonts: fonts, viewport: testViewport)
    var neededFrames = 0
    session.onNeedsFrame = { neededFrames += 1 }
    _ = try session.frame(frameRequest())
    await fonts.settle()
    #expect(network.requests.isEmpty)

    try fonts.apply([fixtureFamily(fixture, named: "Pretendard")])
    await fonts.settle()
    #expect(neededFrames == 1)
    #expect(network.requests.isEmpty)

    var update = try session.frame(frameRequest())
    await fonts.settle()
    #expect(network.requests == [fixture.url("manifest.v2")])

    var inked = false
    var rounds = 0
    while update.geometry.tick != nil, rounds < 16 {
      update = try session.frame(frameRequest())
      inked = inked || setTiles(update).contains { hasInk($0.image) }
      await fonts.settle()
      rounds += 1
    }
    #expect(update.geometry.tick == nil)
    #expect(inked)
    #expect(network.requests.contains(fixture.url("base")))
    #expect(
      Set(network.requests.filter { $0.path().contains("/chunks/") })
        == Set((0..<fixture.chunkCount).map { fixture.url("chunks/\($0)") }))
  }

  @Test func resourceCommitsAskForAFrame() async throws {
    let resources = try await EditorResources.load()
    let fonts = FontLoader(
      target: resources, fetch: FakeFontNetwork().fetch, sleep: { _ in },
      cacheDirectory: cacheDirectory)
    let session = try await EditorSession.open(
      document(), resources: resources, fonts: fonts, viewport: testViewport)
    var requests = 0
    session.onNeedsFrame = { requests += 1 }

    try resources.setTheme(isDark: true)
    #expect(requests == 1)
    try resources.setTheme(isDark: true)
    #expect(requests == 1)

    let update = try session.frame(frameRequest())
    #expect(update.geometry.tick != nil)
  }
}
