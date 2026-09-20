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
  private(set) var documents: [UserGoalDayDocument] = []
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
    return UserGoalDay(day: selectedDay, today: today, history: state.history)
  }

  private var dayQueryInput: KSTDay? {
    hasGoal || state?.history.isEmpty == false ? selectedDay : nil
  }

  func select(_ day: KSTDay) {
    guard day <= today, day != selectedDay else { return }
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
      state = UserGoalState(snapshot: UserGoalSnapshot(user), today: today)
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
