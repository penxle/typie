import Core
import FactoryKit
import Foundation
import GraphQL
import Observation

struct RecentInput: Equatable, Sendable {
  let siteId: String
  let sort: RecentSort
}

@MainActor @Observable
private final class RecentSortSelection {
  var sortOverride: RecentSort?
}

@MainActor @Observable
final class RecentDocumentsStore {
  private(set) var documents: [RecentDocument] = []
  private(set) var fetchedSort: RecentSort?
  private(set) var hasData = false
  private(set) var loadFailed = false

  var sortOverride: RecentSort? {
    get { selection.sortOverride }
    set { selection.sortOverride = newValue }
  }

  @ObservationIgnored var now: () -> Date = { Date() }

  @ObservationIgnored private let layout: HomeLayoutStore
  @ObservationIgnored private let selection = RecentSortSelection()
  @ObservationIgnored private let query: WatchQuery<RecentInput, RecentDocuments_Query>
  @ObservationIgnored private var delivered: RecentDocuments_Query.Data?

  init() {
    let client = Container.shared.graphQLClient()
    let activeSite = Container.shared.activeSite()
    let layout = Container.shared.homeLayoutStore()
    let selection = self.selection
    self.layout = layout
    query = WatchQuery(
      client: client,
      input: {
        activeSite.siteId.map {
          RecentInput(siteId: $0, sort: selection.sortOverride ?? layout.recentSort)
        }
      },
      query: { RecentDocuments_Query(siteId: $0.siteId, sort: $0.sort.graphQL) },
      keepsDataOnInputChange: true)
    keepObserving(while: self) { [weak self] in self?.sync() }
  }

  var sort: RecentSort { selection.sortOverride ?? layout.recentSort }

  var isPlaceholder: Bool { !hasData && !loadFailed }

  var groups: [RecentGroup] {
    RecentGrouping.groups(documents, sort: fetchedSort ?? sort, now: now())
  }

  static let placeholderGroups: [RecentGroup] = [
    RecentGroup(
      id: "today", label: "오늘",
      documents: HomeSiteState.placeholder.recentlyViewed.map {
        RecentDocument(item: $0, viewedAt: nil)
      })
  ]

  func refetch() {
    query.refetch()
  }

  private func sync() {
    let data = query.data
    if let data {
      if data != delivered {
        delivered = data
        documents = data.site.recentDocuments.documents.compactMap {
          RecentDocument.make($0.fragments.recentDocument_document)
        }
        fetchedSort = sort
      }
      hasData = true
      loadFailed = false
    } else {
      delivered = nil
      documents = []
      fetchedSort = nil
      hasData = false
      if !query.isSettled { loadFailed = false }
    }
    if data == nil, query.error != nil || query.isSettled {
      loadFailed = true
    }
  }
}
