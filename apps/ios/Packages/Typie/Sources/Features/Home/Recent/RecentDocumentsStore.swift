import Core
import FactoryKit
import Foundation
import GraphQL
import Observation

@MainActor @Observable
final class RecentDocumentsStore {
  private(set) var documents: [RecentDocument] = []
  private(set) var hasData = false
  private(set) var loadFailed = false

  @ObservationIgnored var now: () -> Date = { Date() }

  @ObservationIgnored private let query: WatchQuery<HomeInput, RecentDocuments_Query>

  init() {
    let client = Container.shared.graphQLClient()
    let activeSite = Container.shared.activeSite()
    query = WatchQuery(
      client: client,
      input: { activeSite.siteId.map(HomeInput.init) },
      query: { RecentDocuments_Query(siteId: $0.siteId) })
    keepObserving(while: self) { [weak self] in self?.sync() }
  }

  var isPlaceholder: Bool { !hasData && !loadFailed }

  var groups: [RecentGroup] {
    RecentGrouping.groups(documents, now: now())
  }

  static let placeholderGroups: [RecentGroup] = [
    RecentGroup(
      id: "today", label: "오늘",
      documents: HomeSiteState.placeholder.recent.map { RecentDocument(item: $0, viewedAt: nil) })
  ]

  func refetch() {
    query.refetch()
  }

  private func sync() {
    let data = query.data
    if let data {
      documents = data.site.recentDocuments.documents.compactMap {
        RecentDocument.make($0.fragments.recentDocument_document)
      }
      hasData = true
      loadFailed = false
    } else {
      documents = []
      hasData = false
      if !query.isSettled { loadFailed = false }
    }
    if data == nil, query.error != nil || query.isSettled {
      loadFailed = true
    }
  }
}
