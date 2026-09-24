import EditorFFI
import Foundation
import Testing

@testable import Editor

@MainActor @Suite final class FontFixtureTests {
  private let cacheDirectory = FileManager.default.temporaryDirectory.appending(
    path: "font-cache-\(UUID().uuidString)")

  deinit {
    try? FileManager.default.removeItem(at: cacheDirectory)
  }

  private func start(_ fixture: FontFixture, _ network: FakeFontNetwork) async throws -> (
    engine: EditorEngine, loader: FontLoader, initialTick: TickResult, commits: CommitRecorder
  ) {
    let resources = try await EditorResources.load()
    let commits = CommitRecorder(resources)
    let loader = FontLoader(
      target: commits, fetch: network.fetch, sleep: { _ in }, cacheDirectory: cacheDirectory)
    try loader.apply([fixture.family])
    let (engine, initialTick) = try await EditorEngine.make(
      resources: resources, document: fontDocument("SUIT", [testParagraph("hello")]),
      viewport: testViewport)
    return (engine, loader, initialTick, commits)
  }

  private func expectEveryPartCommitted(_ commits: CommitRecorder, _ fixture: FontFixture) {
    let chunks = (0..<fixture.chunkCount).map { FontData.chunk(id: UInt16($0)) }
    #expect(commits.parts.count == chunks.count + 2)
    #expect(Array(commits.parts.prefix(2)) == [.manifest, .base])
    #expect(Set(commits.parts.dropFirst(2)) == Set(chunks))
  }

  @Test func fixtureFontLoadsUntilTheEngineStopsAskingForIt() async throws {
    let fixture = try FontFixture()
    let network = FakeFontNetwork(body: fixture.read)
    let (engine, loader, initialTick, commits) = try await start(fixture, network)
    #expect(
      fontRequests(initialTick.events, family: "SUIT") == [
        missing("SUIT", 400, required: [.manifest])
      ])
    let remaining = try await loadFontsUntilQuiet(
      engine: engine, loader: loader, events: initialTick.events, family: "SUIT")
    #expect(remaining.isEmpty)
    #expect(try engine.tick() == nil)
    #expect(network.requests.contains(fixture.url("manifest.v2")))
    #expect(!network.requests.contains(fixture.url("manifest.v1")))
    #expect(network.requests.contains(fixture.url("base")))
    #expect(
      Set(network.requests.filter { $0.path().contains("/chunks/") })
        == Set((0..<fixture.chunkCount).map { fixture.url("chunks/\($0)") }))
    expectEveryPartCommitted(commits, fixture)
  }

  @Test func fixtureFontLoadsFromTheV1ManifestWhenV2IsMissing() async throws {
    let fixture = try FontFixture()
    let network = FakeFontNetwork(body: fixture.read)
    network.fail(fixture.url("manifest.v2"))
    let (engine, loader, initialTick, commits) = try await start(fixture, network)
    let remaining = try await loadFontsUntilQuiet(
      engine: engine, loader: loader, events: initialTick.events, family: "SUIT")
    #expect(remaining.isEmpty)
    #expect(network.requests.contains(fixture.url("manifest.v1")))
    #expect(
      Set(network.requests.filter { $0.path().contains("/chunks/") })
        == Set((0..<fixture.chunkCount).map { fixture.url("chunks/\($0)") }))
    expectEveryPartCommitted(commits, fixture)
  }
}

@MainActor
private final class CommitRecorder: FontCommitTarget {
  private let target: any FontCommitTarget
  private(set) var parts: [FontData] = []

  init(_ target: any FontCommitTarget) {
    self.target = target
  }

  func setFonts(_ families: [FontFamily]) throws {
    try target.setFonts(families)
  }

  func addFont(_ part: FontData, family: String, weight: UInt16, data: Data) throws {
    try target.addFont(part, family: family, weight: weight, data: data)
    parts.append(part)
  }
}
