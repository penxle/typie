public protocol EditingSessionRegistry: Sendable {
  @MainActor func flushSyncAll() async throws
  @MainActor func stopAll() async
}

public protocol OrphanSweeping: Sendable {
  @MainActor func sweep(includeOpenDocuments: Bool, deleteOnSuccess: Bool) async throws
}

public protocol SyncConnectionLifecycle: Sendable {
  func onSessionChanged() async
}

public struct NoopEditingSessionRegistry: EditingSessionRegistry {
  public init() {}

  public func flushSyncAll() async throws {}

  public func stopAll() async {}
}

public struct NoopOrphanSweeper: OrphanSweeping {
  public init() {}

  public func sweep(includeOpenDocuments: Bool, deleteOnSuccess: Bool) async throws {}
}

public struct NoopSyncConnection: SyncConnectionLifecycle {
  public init() {}

  public func onSessionChanged() async {}
}
