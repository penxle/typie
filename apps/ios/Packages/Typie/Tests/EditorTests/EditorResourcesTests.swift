import EditorFFI
import Foundation
import Testing

@testable import Editor

@MainActor @Suite struct EditorResourcesTests {
  private func engine(_ resources: EditorResources) async throws -> EditorEngine {
    try await EditorEngine.make(
      resources: resources, document: testDocument([testParagraph("hello")]),
      viewport: testViewport
    ).engine
  }

  private func initializedEditor(_ resources: EditorResources, _ document: PlainDoc) throws
    -> REditor
  {
    let raw = try resources.createEditor(
      doc: EditorJSON.encode(document), viewport: EditorJSON.encode(testViewport))
    let initialize = try EditorJSON.encode(Message.system(event: .initialize))
    _ = try raw.tickThrough(requestId: raw.enqueueRequest(messages: [initialize]))
    return raw
  }

  @Test func appearanceMapsToTheTwoDefaultThemeVariants() throws {
    #expect(try EditorJSON.encode(EditorResources.themeVariant(isDark: false)) == "\"light-white\"")
    #expect(try EditorJSON.encode(EditorResources.themeVariant(isDark: true)) == "\"dark-black\"")
  }

  @Test func releasedEditorsReceiveNothing() async throws {
    let resources = try await EditorResources.load()
    let log = ReceiptLog()
    var receiver: FakeReceiver? = FakeReceiver(log: log)
    try resources.register(try #require(receiver))
    receiver = nil
    try resources.setTheme(isDark: true)
    #expect(log.count == 0)
  }

  @Test func unchangedCommitNeitherDeliversNorAsksForAFrame() async throws {
    let resources = try await EditorResources.load()
    let receiver = FakeReceiver()
    try resources.register(receiver)
    let engine = try await engine(resources)
    let frames = ReceiptLog()
    engine.onNeedsFrame = { frames.count += 1 }
    try resources.setTheme(isDark: false)
    #expect(receiver.received.isEmpty)
    #expect(frames.count == 0)
  }

  @Test func commitDeliversTheSameUpdateToEveryEditorOnce() async throws {
    let resources = try await EditorResources.load()
    let first = FakeReceiver()
    let second = FakeReceiver()
    try resources.register(first)
    try resources.register(second)
    try resources.setTheme(isDark: true)
    #expect(first.received.count == 1)
    #expect(second.received.count == 1)
    #expect(first.received.first === second.received.first)
  }

  @Test func commitAsksEveryEngineForOneFrame() async throws {
    let resources = try await EditorResources.load()
    let engines = [try await engine(resources), try await engine(resources)]
    let frames = [ReceiptLog(), ReceiptLog()]
    for (engine, log) in zip(engines, frames) {
      engine.onNeedsFrame = { log.count += 1 }
    }
    try resources.setTheme(isDark: true)
    #expect(frames.map(\.count) == [1, 1])
    for engine in engines {
      #expect(try engine.tick() != nil)
    }
  }

  @Test func commitReachesEveryEditorEvenWhenTheCallerIsCancelled() async throws {
    let resources = try await EditorResources.load()
    let first = FakeReceiver()
    let second = FakeReceiver()
    try resources.register(first)
    try resources.register(second)
    let task = Task {
      withUnsafeCurrentTask { $0?.cancel() }
      try resources.setTheme(isDark: true)
    }
    try await task.value
    #expect(first.received.count == 1)
    #expect(second.received.count == 1)
  }

  @Test func registrationCatchesUpWithTheLatestCommit() async throws {
    let resources = try await EditorResources.load()
    let early = FakeReceiver()
    try resources.register(early)
    try resources.setTheme(isDark: true)
    let late = FakeReceiver()
    try resources.register(late)
    #expect(late.received.count == 1)
    #expect(late.received.first === early.received.first)
  }

  @Test func failedCatchUpLeavesTheReceiverUnregistered() async throws {
    let resources = try await EditorResources.load()
    try resources.setTheme(isDark: true)
    let receiver = FakeReceiver()
    receiver.failing = true
    #expect(throws: FakeFailure.self) { try resources.register(receiver) }
    receiver.failing = false
    try resources.setTheme(isDark: false)
    #expect(receiver.received.isEmpty)
  }

  @Test func engineBuiltAfterACommitCatchesUpWithoutFailing() async throws {
    let resources = try await EditorResources.load()
    try resources.setTheme(isDark: true)
    let engine = try await engine(resources)
    #expect(try engine.tick() == nil)
    try resources.setTheme(isDark: false)
    #expect(try engine.tick() != nil)
  }

  @Test func catchUpAppliesACommitThatLandedAfterTheSnapshotWasTaken() async throws {
    let resources = try await EditorResources.load()
    let raw = try initializedEditor(resources, testDocument([testParagraph("hello")]))
    try resources.setTheme(isDark: true)
    let receiver = RawReceiver(raw)
    try resources.register(receiver)
    #expect(try raw.tick() != nil)
    try resources.setTheme(isDark: false)
    #expect(try raw.tick() != nil)
  }

  @Test(arguments: [false, true])
  func catchUpRequestsTheDocumentFontAfterAFontListCommit(followedByThemeCommit: Bool)
    async throws
  {
    let resources = try await EditorResources.load()
    let family = FontFamily(
      name: "SUIT", source: .default, weights: [FontWeight(value: 400, hash: "test")])
    let document = PlainDoc(
      root: PlainNodeEntry(
        node: .root(PlainRootNode(layoutMode: paginatedLayout)),
        modifiers: [.fontFamily: .fontFamily(value: family.name)],
        children: [testParagraph("hello")]))
    let raw = try initializedEditor(resources, document)
    let families = [try EditorJSON.encode(family)]
    try resources.commit { try $0.setFonts(families: families) }
    if followedByThemeCommit {
      try resources.setTheme(isDark: true)
    }
    let receiver = RawReceiver(raw)
    try resources.register(receiver)
    let json = try #require(try raw.tick())
    let tick = try EditorJSON.decode(TickResult.self, from: json)
    let requested = tick.events.compactMap { event -> [FontData]? in
      guard case .fontDataMissing(family.name, 400, let required, _) = event else { return nil }
      return required
    }
    #expect(requested == [[.manifest]])
  }

  @Test func failingEditorIsDroppedWhileOthersKeepReceiving() async throws {
    let resources = try await EditorResources.load()
    let failing = FakeReceiver()
    let healthy = FakeReceiver()
    try resources.register(failing)
    try resources.register(healthy)
    failing.failing = true
    try resources.setTheme(isDark: true)
    failing.failing = false
    try resources.setTheme(isDark: false)
    #expect(failing.received.isEmpty)
    #expect(healthy.received.count == 2)
  }
}
