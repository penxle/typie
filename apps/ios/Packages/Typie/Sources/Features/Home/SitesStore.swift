import Core
import FactoryKit
import Foundation
import GraphQL
import Observation

@MainActor @Observable
final class SitesStore {
  private(set) var sites: [Site] = []
  private(set) var isCreating = false
  private(set) var loadFailed = false

  @ObservationIgnored var onCreated: @MainActor () -> Void = {}

  @ObservationIgnored private let client: any GraphQLClient
  @ObservationIgnored private let query: WatchQuery<NoInput, SiteSwitcher_Query>
  @ObservationIgnored private let activeSite = Container.shared.activeSite()
  @ObservationIgnored private var pendingCreatedSiteId: String?
  @ObservationIgnored private var pendingCreation: CheckedContinuation<Bool, Never>?

  init() {
    let client = Container.shared.graphQLClient()
    self.client = client
    query = WatchQuery(client: client, query: SiteSwitcher_Query())
    keepObserving(while: self) { [weak self] in self?.sync() }
  }

  deinit {
    pendingCreation?.resume(returning: false)
  }

  var current: Site? {
    sites.first { $0.id == activeSite.siteId }
  }

  var isSettled: Bool { query.isSettled }

  func refetch() {
    query.refetch()
  }

  func select(_ id: String) {
    activeSite.select(id)
  }

  func create(name: String) async -> Bool {
    guard !isCreating else { return false }
    isCreating = true
    defer { isCreating = false }
    let trimmed = name.trimmingCharacters(in: .whitespacesAndNewlines)
    let id: String
    do {
      id = try await client.perform(
        SiteSwitcher_CreateSite_Mutation(
          input: CreateSiteInput(name: trimmed.isEmpty ? "새 스페이스" : trimmed))
      ).createSite.id
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
    let fetched = data?.me?.sites
    if let fetched {
      let sites = fetched.map {
        Site(
          id: $0.id, name: $0.name, url: $0.url,
          logo: $0.logo.fragments.img_image)
      }
      self.sites = sites
      loadFailed = false
      reconcile(sites)
    }
    if error != nil, pendingCreation != nil {
      failPendingCreation()
    }
    if error != nil, fetched?.isEmpty ?? true {
      loadFailed = true
    }
  }

  private func reconcile(_ sites: [Site]) {
    let available = sites.map(\.id)
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
    if let resolved = ActiveSiteStore.resolve(stored: activeSite.siteId, available: available),
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
