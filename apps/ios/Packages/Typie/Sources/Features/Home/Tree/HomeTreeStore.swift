import Core
import FactoryKit
import GraphQL
import Observation

struct HomeTreeChildrenInput: Equatable, Sendable {
  let entityId: String
}

struct HomeTreeExpansion: Equatable, Sendable {
  let expanded: Set<String>
  let children: [String: [HomeTreeNode]]
}

@MainActor @Observable
final class HomeTreeStore {
  private(set) var expanded: Set<String> = []

  @ObservationIgnored private let client: any GraphQLClient
  @ObservationIgnored private let preferences: UserPreferences
  @ObservationIgnored private let activeSite: ActiveSiteStore

  private var queries: [String: WatchQuery<HomeTreeChildrenInput, HomeTree_Children_Query>] = [:]
  private var childrenById: [String: [HomeTreeNode]] = [:]
  private var parentById: [String: HomeTreeNode] = [:]
  @ObservationIgnored private var loadedSiteId: String??

  init() {
    client = Container.shared.graphQLClient()
    preferences = Container.shared.userPreferences()
    activeSite = Container.shared.activeSite()
    keepObserving(while: self) { [weak self] in self?.syncSite() }
  }

  var expansion: HomeTreeExpansion {
    HomeTreeExpansion(
      expanded: expanded, children: childrenById.filter { expanded.contains($0.key) })
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
    let query = WatchQuery(
      client: client,
      input: { HomeTreeChildrenInput(entityId: id) },
      query: { HomeTree_Children_Query(entityId: $0.entityId) })
    queries[id] = query
    keepObserving(while: self) { [weak self, weak query] in
      guard let self, let query, queries[id] === query else { return }
      guard let entity = query.data?.entity else { return }
      let children = entity.children.compactMap { HomeTreeNode.make($0.fragments.homeTree_entity) }
      let parent = HomeTreeNode.make(entity.fragments.homeTree_entity)
      guard childrenById[id] != children || parentById[id] != parent else { return }
      childrenById[id] = children
      parentById[id] = parent
    }
  }

  func children(of id: String) -> [HomeTreeNode] {
    childrenById[id] ?? []
  }

  func node(_ id: String) -> HomeTreeNode? {
    parentById[id]
  }

  func isLoaded(_ id: String) -> Bool {
    childrenById[id] != nil
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
    childrenById = [:]
    parentById = [:]
    expanded = siteId.map { Set(preferences.expandedFolders(siteId: $0)) } ?? []
    for id in expanded.sorted() { ensureLoaded(id) }
  }

  private func persist() {
    guard case .some(let siteId?) = loadedSiteId else { return }
    preferences.setExpandedFolders(expanded.sorted(), siteId: siteId)
  }
}
