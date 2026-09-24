import EditorFFI
import Foundation
import Testing

@testable import Editor

@MainActor @Suite struct EditorEngineTests {
  private func engine(_ children: [PlainNodeEntry]) async throws -> EditorEngine {
    try await EditorEngine.make(
      resources: EditorResources.load(), document: testDocument(children),
      viewport: testViewport
    ).engine
  }

  private func ime(_ engine: EditorEngine) throws -> Ime {
    let json = try #require(try engine.raw.ime(beforeLimit: 4096, afterLimit: 4096))
    return try EditorJSON.decode(Ime.self, from: json)
  }

  private func selectAll(_ engine: EditorEngine) throws {
    _ = try engine.send([
      .system(event: .setFocused(focused: true)), .selection(op: .expand(unit: .all)),
    ])
  }

  @Test func initializesAndReportsPageGeometry() async throws {
    let engine = try await engine([testParagraph("hello")])
    #expect(engine.revision.value > 0)
    #expect(try engine.pageSizes() == [Size(width: 794, height: 1123)])
  }

  @Test func initializesAnEmptyDocument() async throws {
    let engine = try await engine([])
    #expect(try engine.pageSizes().count == 1)
  }

  @Test func tickThroughRecordsTheLatestRevision() async throws {
    let engine = try await engine([testParagraph("hello")])
    let before = engine.revision
    let result = try engine.send([.system(event: .setFocused(focused: true))])
    #expect(result.revision == engine.revision)
    #expect(engine.revision.value > before.value)
  }

  @Test func tickRecordsTheRevisionOfQueuedMessages() async throws {
    let engine = try await engine([testParagraph("hello")])
    let before = engine.revision
    _ = try engine.enqueue([.system(event: .setFocused(focused: true))])
    let result = try #require(try engine.tick())
    #expect(result.revision == engine.revision)
    #expect(engine.revision.value > before.value)
  }

  @Test func tickReturnsNilWhenNothingIsQueued() async throws {
    let engine = try await engine([testParagraph("hello")])
    let before = engine.revision
    #expect(try engine.tick() == nil)
    #expect(engine.revision == before)
  }

  @Test func nonAsciiTextRoundTripsThroughTheEngine() async throws {
    let engine = try await engine([testParagraph("가"), testParagraph("다")])
    try selectAll(engine)
    let window = try ime(engine)
    #expect(window.text.contains("가"))
    #expect(window.text.unicodeScalars.contains { $0 == "\u{2028}" || $0 == "\u{2029}" })
    let scalars = Array(window.text.unicodeScalars)
    let first = try #require(scalars.firstIndex(of: "가"))
    let offset = window.windowStart + first + 1
    _ = try engine.send([
      .selection(op: .setFlat(start: offset, end: offset)),
      .textInput(ops: [.replaceSelection(text: "나😀")]),
    ])
    #expect(try ime(engine).text.unicodeScalars.contains("가나😀".unicodeScalars))
  }

  @Test func enumKeyedModifiersAndUnitPayloadsRoundTrip() async throws {
    let engine = try await engine([
      testParagraph("굵게", modifiers: [.bold: .bold], carry: [.bold])
    ])
    try selectAll(engine)
    let json = try #require(try engine.raw.modifierState())
    let state = try EditorJSON.decode(ModifierState.self, from: json)
    #expect(state.bold == .uniform(value: FFIUnit()))
  }

  @Test func rootAttributesRoundTrip() async throws {
    let engine = try await engine([testParagraph("hello")])
    let root = try EditorJSON.decode(PlainRootNode.self, from: engine.raw.rootAttrs())
    #expect(root == PlainRootNode(layoutMode: paginatedLayout))
  }

  @Test func omittedOptionalFieldsDecodeAsAbsent() async throws {
    let engine = try await engine([testParagraph("hello")])
    try selectAll(engine)
    let json = try #require(try engine.raw.copySelection())
    let payload = try EditorJSON.decode(ClipboardPayload.self, from: json)
    #expect(payload.text.contains("hello"))
    #expect(payload.dragGhost == nil)
  }

  @Test func byteFieldsDecodeFromNumberArrays() async throws {
    let engine = try await engine([testParagraph("hello")])
    let json = try engine.raw.missingChangesetsTolerant(remoteHeadsPayload: Data())
    let missing = try EditorJSON.decode(MissingChangesets.self, from: json)
    #expect(!missing.bytes.isEmpty)
  }

  private func recentEditBaseline(_ edit: ([UInt8]) -> [UInt8]) async throws -> UInt32 {
    let engine = try await engine([testParagraph("hello")])
    let heads = [UInt8](try engine.raw.currentHeads())
    try #require(!heads.isEmpty)
    try engine.raw.enableRecentEdits(nowMs: 7_200_000, windowMs: 3_600_000)
    let bucket = try EditorJSON.encode(RecentHeadBucket(atMs: 3_600_000, heads: edit(heads)))
    return try engine.raw.setRecentEditBaseline(nowMs: 7_200_000, buckets: [bucket])
  }

  @Test func byteFieldsEncodeToNumberArraysTheEngineDecodes() async throws {
    let accepted = try await recentEditBaseline { $0 }
    #expect(accepted == 1)
  }

  @Test func corruptedByteFieldsAreRejectedByTheEngine() async throws {
    let accepted = try await recentEditBaseline { heads in
      var corrupted = heads
      corrupted[corrupted.count - 1] ^= 0xFF
      return corrupted
    }
    #expect(accepted == 0)
  }
}
