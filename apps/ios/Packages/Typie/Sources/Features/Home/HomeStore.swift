import Core
import FactoryKit
import Foundation
import GraphQL
import Observation

struct HomeInput: Equatable, Sendable {
  let siteId: String
}

@MainActor @Observable
final class HomeStore {
  private(set) var goal: UserGoalState?
  private(set) var site: HomeSiteState?
  private(set) var hasData = false
  private(set) var loadFailed = false

  @ObservationIgnored var now: () -> Date = { Date() }

  @ObservationIgnored private let activeSite: ActiveSiteStore
  @ObservationIgnored private let query: WatchQuery<HomeInput, HomeScreen_Query>

  init() {
    let client = Container.shared.graphQLClient()
    let activeSite = Container.shared.activeSite()
    self.activeSite = activeSite
    query = WatchQuery(
      client: client,
      input: { activeSite.siteId.map(HomeInput.init) },
      query: { HomeScreen_Query(siteId: $0.siteId) })
    keepObserving(while: self) { [weak self] in self?.sync() }
  }

  var siteId: String? { activeSite.siteId }

  var isSettled: Bool { query.isSettled }

  var previewsPlaceholder = false

  var isPlaceholder: Bool { previewsPlaceholder || (!hasData && !loadFailed) }

  var goalOrPlaceholder: UserGoalState {
    if previewsPlaceholder { return UserGoalState.placeholder }
    return goal ?? UserGoalState.placeholder
  }

  var siteOrPlaceholder: HomeSiteState {
    if previewsPlaceholder { return HomeSiteState.placeholder }
    return site ?? HomeSiteState.placeholder
  }

  func refetch() {
    query.refetch()
  }

  private func sync() {
    let data = query.data
    let error = query.error
    let user = data?.me?.fragments.userGoalSection_user
    if let data, let user {
      goal = UserGoalState(snapshot: UserGoalSnapshot(user), today: KSTDay(now()))
      site = HomeSiteState(data.site.fragments.homeScreen_site)
      hasData = true
      loadFailed = false
    }
    if data == nil {
      goal = nil
      site = nil
      hasData = false
      if !query.isSettled { loadFailed = false }
    }
    if user == nil, !hasData, error != nil || query.isSettled {
      loadFailed = true
    }
  }
}
