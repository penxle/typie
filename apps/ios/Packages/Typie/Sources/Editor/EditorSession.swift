internal import EditorFFI

@MainActor
final class EditorSession {
  let engine: EditorEngine
  var onNeedsFrame: (() -> Void)?

  private let source: EditorFrameSource
  private let fonts: FontLoader

  private init(engine: EditorEngine, fonts: FontLoader) throws {
    self.engine = engine
    self.fonts = fonts
    source = try EditorFrameSource(engine: engine)
    engine.onNeedsFrame = { [weak self] in self?.onNeedsFrame?() }
  }

  static func open(
    _ document: EditorDocument, resources: EditorResources, fonts: FontLoader, viewport: Viewport
  ) async throws -> EditorSession {
    let (engine, initialTick) = try await EditorEngine.make(
      resources: resources, document: document.plain, viewport: viewport)
    let session = try EditorSession(engine: engine, fonts: fonts)
    fonts.receive(initialTick.events)
    return session
  }

  func frame(_ request: ViewportRequest) throws -> EditorFrameUpdate {
    let update = try source.frame(request)
    if let tick = update.geometry.tick {
      fonts.receive(tick.events)
    }
    return update
  }

  func fill(budgetMs: Double) throws -> EditorFrameUpdate? {
    try source.fill(budgetMs: budgetMs)
  }

  func presented(_ id: UInt64) throws {
    try source.presented(id)
  }
}
