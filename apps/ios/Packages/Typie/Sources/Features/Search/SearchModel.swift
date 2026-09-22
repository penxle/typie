import Core
import FactoryKit
import Foundation
import GraphQL
import Observation

struct SearchInput: Equatable, Sendable {
  let siteId: String
  let keyword: String
}

@MainActor @Observable
final class SearchTerms {
  var keyword = ""
}

@MainActor @Observable
final class SearchModel {
  enum Content: Equatable {
    case recent
    case pending
    case results([SearchHit])
    case empty
    case failed
  }

  private(set) var text = ""
  private(set) var recentSearches: [String]
  private(set) var hits: [SearchHit]?

  @ObservationIgnored private let terms = SearchTerms()
  @ObservationIgnored private let query: WatchQuery<SearchInput, SearchScreen_Search_Query>
  @ObservationIgnored private let preferences: UserPreferences
  @ObservationIgnored private let debounce: Duration
  @ObservationIgnored private var debounceTask: Task<Void, Never>?

  init(debounce: Duration = .milliseconds(300)) {
    let preferences = Container.shared.userPreferences()
    let activeSite = Container.shared.activeSite()
    self.preferences = preferences
    self.debounce = debounce
    recentSearches = preferences.recentSearches
    let terms = self.terms
    query = WatchQuery(
      client: Container.shared.graphQLClient(),
      input: {
        let keyword = terms.keyword
        guard let siteId = activeSite.siteId, !Self.isBlank(keyword) else { return nil }
        return SearchInput(siteId: siteId, keyword: keyword)
      },
      query: { SearchScreen_Search_Query(siteId: $0.siteId, query: $0.keyword) },
      keepsDataOnInputChange: true)
    keepObserving(while: self) { [weak self] in self?.sync() }
  }

  var isSearching: Bool {
    !Self.isBlank(terms.keyword) && !query.isSettled
  }

  var content: Content {
    if Self.isBlank(text) { return .recent }
    if let hits {
      return hits.isEmpty ? .empty : .results(hits)
    }
    if query.error != nil { return .failed }
    return .pending
  }

  func setText(_ text: String) {
    self.text = text
    debounceTask?.cancel()
    if Self.isBlank(text) {
      terms.keyword = ""
      return
    }
    debounceTask = Task { [weak self, debounce] in
      try? await Task.sleep(for: debounce)
      guard !Task.isCancelled, let self else { return }
      terms.keyword = text
    }
  }

  func submit() {
    debounceTask?.cancel()
    terms.keyword = text
    record(terms.keyword)
  }

  func select(recent keyword: String) {
    text = keyword
    submit()
  }

  func remove(recent keyword: String) {
    recentSearches = RecentSearches.removing(keyword, from: recentSearches)
    preferences.recentSearches = recentSearches
  }

  func didOpen(_ hit: SearchHit) {
    record(terms.keyword)
  }

  func didInteract() {
    record(terms.keyword)
  }

  private func record(_ keyword: String) {
    recentSearches = RecentSearches.adding(keyword, to: recentSearches)
    preferences.recentSearches = recentSearches
  }

  private func sync() {
    let data = query.data
    hits = data.map { SearchHit.make(from: $0.search.hits) }
  }

  private static func isBlank(_ text: String) -> Bool {
    text.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty
  }
}
