import Core
import FactoryKit
import GraphQL
import Observation

struct HomeTreeChildrenInput: Equatable, Sendable {
  let entityId: String
}

@MainActor @Observable
final class HomeTreeStore {
  private(set) var expanded: Set<String> = []

  @ObservationIgnored private let client: any GraphQLClient
  @ObservationIgnored private let preferences: UserPreferences
  @ObservationIgnored private let activeSite: ActiveSiteStore
  private var queries: [String: WatchQuery<HomeTreeChildrenInput, HomeTree_Children_Query>] = [:]
  @ObservationIgnored private var loadedSiteId: String??

  init() {
    client = Container.shared.graphQLClient()
    preferences = Container.shared.userPreferences()
    activeSite = Container.shared.activeSite()
    keepObserving(while: self) { [weak self] in self?.syncSite() }
  }

  func isExpanded(_ id: String) -> Bool {
    expanded.contains(id)
  }

  func toggle(_ id: String) {
    if expanded.contains(id) {
      if let query = queries[id], query.error != nil, query.data == nil {
        query.refetch()
        return
      }
      expanded.remove(id)
    } else {
      expanded.insert(id)
      ensureLoaded(id)
    }
    persist()
  }

  func ensureLoaded(_ id: String) {
    guard queries[id] == nil else { return }
    queries[id] = WatchQuery(
      client: client,
      input: { HomeTreeChildrenInput(entityId: id) },
      query: { HomeTree_Children_Query(entityId: $0.entityId) })
  }

  func children(of id: String) -> [HomeTreeNode] {
    guard let entity = queries[id]?.data?.entity else { return [] }
    return entity.children.compactMap { HomeTreeNode.make($0.fragments.homeTree_entity) }
  }

  func node(_ id: String) -> HomeTreeNode? {
    guard let entity = queries[id]?.data?.entity else { return nil }
    return HomeTreeNode.make(entity.fragments.homeTree_entity)
  }

  func isLoaded(_ id: String) -> Bool {
    queries[id]?.data != nil
  }

  func isLoading(_ id: String) -> Bool {
    guard let query = queries[id] else { return false }
    return !query.isSettled && query.data == nil
  }

  func failed(_ id: String) -> Bool {
    guard let query = queries[id] else { return false }
    return query.error != nil && query.data == nil
  }

  func refetchChildren(of id: String) {
    queries[id]?.refetch()
  }

  private func syncSite() {
    let siteId = activeSite.siteId
    guard loadedSiteId != .some(siteId) else { return }
    loadedSiteId = .some(siteId)
    queries = [:]
    expanded = siteId.map { Set(preferences.expandedFolders(siteId: $0)) } ?? []
    for id in expanded.sorted() { ensureLoaded(id) }
  }

  private func persist() {
    guard case .some(let siteId?) = loadedSiteId else { return }
    preferences.setExpandedFolders(expanded.sorted(), siteId: siteId)
  }
}
