import Core
import Foundation
import Observation

@MainActor @Observable
public final class SpaceSwitcherModel {
  public private(set) var spaces: [Space] = []
  public private(set) var isCreating = false
  public private(set) var loadFailed = false

  @ObservationIgnored private let query: WatchQuery<NoInput, SpaceSwitcher_Query>
  @ObservationIgnored private let activeSite: ActiveSiteStore
  @ObservationIgnored private let createSite: @Sendable (String) async throws -> String
  @ObservationIgnored private let onCreated: @MainActor () -> Void
  @ObservationIgnored private var pendingCreatedSiteId: String?
  @ObservationIgnored private var pendingCreation: CheckedContinuation<Bool, Never>?

  public init(
    query: WatchQuery<NoInput, SpaceSwitcher_Query>,
    activeSite: ActiveSiteStore,
    createSite: @escaping @Sendable (String) async throws -> String,
    onCreated: @escaping @MainActor () -> Void
  ) {
    self.query = query
    self.activeSite = activeSite
    self.createSite = createSite
    self.onCreated = onCreated
    keepObserving(while: self) { [weak self] in self?.sync() }
  }

  deinit {
    pendingCreation?.resume(returning: false)
  }

  public var current: Space? {
    spaces.first { $0.id == activeSite.siteId }
  }

  public var isSettled: Bool { query.isSettled }

  public func refetch() {
    query.refetch()
  }

  public func retry() {
    query.refetch()
  }

  public func select(_ id: String) {
    activeSite.select(id)
  }

  public func createSpace(name: String) async -> Bool {
    guard !isCreating else { return false }
    isCreating = true
    defer { isCreating = false }
    let trimmed = name.trimmingCharacters(in: .whitespacesAndNewlines)
    let id: String
    do {
      id = try await createSite(trimmed.isEmpty ? "새 스페이스" : trimmed)
    } catch {
      return false
    }
    return await withCheckedContinuation { continuation in
      pendingCreation?.resume(returning: false)
      pendingCreatedSiteId = id
      pendingCreation = continuation
      query.refetch()
    }
  }

  private func sync() {
    let data = query.data
    let error = query.error
    let sites = data?.me?.sites
    if let sites {
      let spaces = sites.map {
        Space(
          id: $0.id, name: $0.name, url: $0.url,
          logo: ImageSource($0.logo.fragments.tImage_image)?.url)
      }
      self.spaces = spaces
      loadFailed = false
      reconcile(spaces)
    }
    if error != nil, pendingCreation != nil {
      failPendingCreation()
    }
    if error != nil, sites?.isEmpty ?? true {
      loadFailed = true
    }
  }

  private func reconcile(_ spaces: [Space]) {
    let available = spaces.map(\.id)
    if let pending = pendingCreatedSiteId {
      if available.contains(pending) {
        let continuation = pendingCreation
        pendingCreatedSiteId = nil
        pendingCreation = nil
        activeSite.select(pending)
        onCreated()
        continuation?.resume(returning: true)
        return
      }
      failPendingCreation()
    }
    if let resolved = resolveActiveSiteId(stored: activeSite.siteId, available: available),
      resolved != activeSite.siteId
    {
      activeSite.select(resolved)
    }
  }

  private func failPendingCreation() {
    let continuation = pendingCreation
    pendingCreatedSiteId = nil
    pendingCreation = nil
    continuation?.resume(returning: false)
  }
}
