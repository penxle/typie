import Core
import FactoryKit
import Foundation
import GraphQL
import Observation

@MainActor @Observable
final class UserGoalModel {
  private(set) var state: UserGoalState?
  private(set) var hasData = false
  private(set) var loadFailed = false
  private(set) var isMutating = false
  private var selectedDayOverride: KSTDay?
  private var displayedWeekOverride: KSTDay?
  private var anchorOverride: KSTDay?
  private(set) var documents: [UserGoalDayDocument] = []
  private(set) var dailyAdditions: [KSTDay: Int] = [:]
  private(set) var documentsFailed = false

  @ObservationIgnored var now: () -> Date = { Date() }

  @ObservationIgnored private let client: any GraphQLClient
  @ObservationIgnored private let query: WatchQuery<NoInput, UserGoalScreen_Query>
  @ObservationIgnored private var dayQuery: WatchQuery<KSTDay, UserGoalDay_Query>!

  init() {
    let client = Container.shared.graphQLClient()
    self.client = client
    query = WatchQuery(client: client, query: UserGoalScreen_Query())
    dayQuery = WatchQuery(
      client: client, input: { [weak self] in self?.dayQueryInput },
      query: { day in UserGoalDay_Query(date: formatDateTime(day.date)) })
    keepObserving(while: self) { [weak self] in self?.sync() }
    keepObserving(while: self) { [weak self] in self?.syncDocuments() }
  }

  var isSettled: Bool { query.isSettled }
  var hasGoal: Bool { state?.hasGoal == true }
  var today: KSTDay { KSTDay(now()) }
  var selectedDay: KSTDay { selectedDayOverride ?? today }

  var selected: UserGoalDay? {
    guard let state else { return nil }
    let day = UserGoalDay(day: selectedDay, today: today, history: state.history)
    guard !day.hasGoal else { return day }
    return UserGoalDay(
      day: day.day, isToday: day.isToday, target: nil,
      additions: dailyAdditions[day.day] ?? 0, achieved: false)
  }

  var displayedWeek: KSTDay { displayedWeekOverride ?? UserGoalMonth.weekStart(of: today) }
  var anchor: KSTDay { anchorOverride ?? today }

  func grid(for week: KSTDay) -> UserGoalMonth? {
    guard let state else { return nil }
    let containing = week == displayedWeek ? anchor : week
    let key = KSTDay(year: containing.year, month: containing.month, day: 1)
    let today = today
    if gridToday != today {
      gridToday = today
      gridCache = [:]
    }
    if let cached = gridCache[key] { return cached }
    let grid = UserGoalMonth(
      history: state.history, today: today, containing: containing, coverageStart: coverageStart)
    gridCache[key] = grid
    return grid
  }

  @ObservationIgnored private var gridToday: KSTDay?
  @ObservationIgnored private var gridCache: [KSTDay: UserGoalMonth] = [:]

  private static let historyDays = 365

  private var coverageStart: KSTDay? {
    state == nil ? nil : today.adding(days: -(Self.historyDays - 1))
  }

  var weeks: [KSTDay] {
    guard let coverageStart else { return [] }
    var weeks: [KSTDay] = []
    var cursor = UserGoalMonth.weekStart(of: coverageStart)
    let last = UserGoalMonth.weekStart(of: today)
    while cursor <= last {
      weeks.append(cursor)
      cursor = cursor.adding(days: 7)
    }
    return weeks
  }

  var monthPages: [KSTDay] {
    let weeks = weeks
    guard let first = weeks.first, let last = weeks.last else { return [] }
    var pages: [KSTDay] = []
    var month = KSTDay(year: first.year, month: first.month, day: 1)
    while month <= last {
      if (month.year, month.month) == (anchor.year, anchor.month) {
        pages.append(displayedWeek)
      } else {
        let start = UserGoalMonth.weekStart(of: month)
        let representative = start == month ? start : start.adding(days: 7)
        if representative >= first, representative <= last { pages.append(representative) }
      }
      month = UserGoalMonth.lastDay(year: month.year, month: month.month).adding(days: 1)
    }
    return pages
  }

  func show(week: KSTDay) {
    displayedWeekOverride = week
    anchorOverride = week
  }

  func settleDisplayedWeek() {
    let week = UserGoalMonth.weekStart(of: anchor)
    if week != displayedWeek { displayedWeekOverride = week }
  }

  private var dayQueryInput: KSTDay? {
    hasData ? selectedDay : nil
  }

  func select(_ day: KSTDay) {
    guard day <= today else { return }
    anchorOverride = day
    guard day != selectedDay else { return }
    selectedDayOverride = day
  }

  func refetch() {
    query.refetch()
  }

  func save(target: Int) async -> Bool {
    await mutate {
      _ = try await client.perform(
        UserGoalScreen_UpdateUserGoal_Mutation(
          input: UpdateUserGoalInput(targetCharacterCount: Int32(target))))
    }
  }

  func remove() async -> Bool {
    await mutate { _ = try await client.perform(UserGoalScreen_DeleteUserGoal_Mutation()) }
  }

  private func mutate(_ operation: () async throws -> Void) async -> Bool {
    guard !isMutating else { return false }
    isMutating = true
    defer { isMutating = false }
    do {
      try await operation()
    } catch {
      return false
    }
    return true
  }

  private func sync() {
    let data = query.data
    let error = query.error
    let user = data?.me?.fragments.userGoalSection_user
    if let user {
      dailyAdditions = Dictionary(
        (data?.me?.characterCountChanges ?? []).compactMap { row in
          parseDateTime(row.date).map { (KSTDay($0), row.additions) }
        }, uniquingKeysWith: { _, last in last })
      state = UserGoalState(snapshot: UserGoalSnapshot(user), today: today)
      gridCache = [:]
      hasData = true
      loadFailed = false
    }
    if error != nil, user == nil, !hasData {
      loadFailed = true
    }
  }

  private func syncDocuments() {
    let data = dayQuery.data
    let error = dayQuery.error
    if let rows = data?.me?.dailyDocumentCharacterCountChanges {
      documents = rows.map(UserGoalDayDocument.init)
      documentsFailed = false
    } else if error != nil {
      documents = []
      documentsFailed = true
    } else {
      documents = []
      documentsFailed = false
    }
  }
}
