import Core
import FactoryKit
import Foundation
import GraphQL
import Observation

struct SiteEntitiesInput: Equatable, Sendable {
  let siteId: String
}

struct SiteEntitiesState: Equatable, Sendable {
  let name: String
  let folderCount: Int
  let documentCount: Int
  let items: [EntityContainerItem]

  init(name: String, folderCount: Int, documentCount: Int, items: [EntityContainerItem]) {
    self.name = name
    self.folderCount = folderCount
    self.documentCount = documentCount
    self.items = items
  }

  init(_ site: SiteEntities_Query.Data.Site) {
    name = site.name
    folderCount = site.folderCount
    documentCount = site.documentCount
    items = site.entities.compactMap { EntityContainerItem.make($0.fragments.entityRow_entity) }
  }

  var summary: String {
    EntityText.spaceSummary(folders: folderCount, documents: documentCount)
  }
}

@MainActor @Observable
final class SiteEntitiesStore {
  private(set) var site: SiteEntitiesState?
  private(set) var hasData = false
  private(set) var loadFailed = false

  @ObservationIgnored var now: () -> Date = { Date() }

  @ObservationIgnored private let query: WatchQuery<SiteEntitiesInput, SiteEntities_Query>

  init() {
    let client = Container.shared.graphQLClient()
    let activeSite = Container.shared.activeSite()
    query = WatchQuery(
      client: client,
      input: { activeSite.siteId.map { SiteEntitiesInput(siteId: $0) } },
      query: { SiteEntities_Query(siteId: $0.siteId) })
    keepObserving(while: self) { [weak self] in self?.sync() }
  }

  var isPlaceholder: Bool { !hasData && !loadFailed }

  func refetch() {
    query.refetch()
  }

  private func sync() {
    let data = query.data
    if let data {
      let next = SiteEntitiesState(data.site)
      if site != next { site = next }
      hasData = true
      loadFailed = false
    } else {
      site = nil
      hasData = false
      if !query.isSettled { loadFailed = false }
    }
    if data == nil, query.error != nil || query.isSettled {
      loadFailed = true
    }
  }
}
