internal import EditorFFI
import Foundation

@MainActor
final class EditorEngine {
  private var editor: REditor
  private(set) var revision: Revision
  var onNeedsFrame: (() -> Void)?

  var raw: REditor { editor }

  private init(raw: REditor, revision: Revision) {
    editor = raw
    self.revision = revision
  }

  static func make(resources: EditorResources, document: PlainDoc, viewport: Viewport)
    async throws -> (engine: EditorEngine, initialTick: TickResult)
  {
    let documentJSON = try EditorJSON.encode(document)
    let viewportJSON = try EditorJSON.encode(viewport)
    let initializeJSON = try EditorJSON.encode(Message.system(event: .initialize))
    let (raw, tickJSON) = try await Task.detached(priority: .userInitiated) {
      let raw = try resources.createEditor(doc: documentJSON, viewport: viewportJSON)
      let request = try raw.enqueueRequest(messages: [initializeJSON])
      return (raw, try raw.tickThrough(requestId: request))
    }.value
    let tick = try EditorJSON.decode(TickResult.self, from: tickJSON)
    let engine = EditorEngine(raw: raw, revision: tick.revision)
    try resources.register(engine)
    return (engine, tick)
  }

  func enqueue(_ messages: [Message]) throws -> RequestId {
    let json = try raw.enqueueRequest(messages: messages.map { try EditorJSON.encode($0) })
    return try EditorJSON.decode(RequestId.self, from: json)
  }

  func tick() throws -> TickResult? {
    guard let json = try raw.tick() else { return nil }
    return try record(json)
  }

  func tickThrough(_ request: RequestId) throws -> TickResult {
    try record(raw.tickThrough(requestId: EditorJSON.encode(request)))
  }

  func send(_ messages: [Message]) throws -> TickResult {
    try tickThrough(enqueue(messages))
  }

  func pageSizes() throws -> [Size] {
    try raw.pageSizes().map { try EditorJSON.decode(Size.self, from: $0) }
  }

  private func record(_ json: String) throws -> TickResult {
    let result = try EditorJSON.decode(TickResult.self, from: json)
    revision = result.revision
    return result
  }
}

extension EditorEngine: ResourceReceiver {
  func receiveResourceUpdate(_ update: RResourceUpdate) throws {
    try raw.receiveResourceUpdate(update: update)
    onNeedsFrame?()
  }
}
