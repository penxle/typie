import Core
import FactoryKit
import FactoryTesting
import Foundation
import GraphQL
import Testing

@testable import Features

@MainActor
@Suite(.container) struct UserGoalModelTests {
  private static let now = Date(timeIntervalSince1970: 1_790_000_000)
  private static let today = KSTDay(now)

  private func makeModel() -> (UserGoalModel, FakeGraphQLClient) {
    let client = FakeGraphQLClient()
    registerFake(client)
    let model = Container.shared.userGoalModel()
    model.now = { Self.now }
    return (model, client)
  }

  private func iso(_ day: KSTDay) -> String {
    ISO8601DateFormatter().string(from: day.date)
  }

  @Test func withoutGoalExposesHistoryStreaksAndNoStatus() async throws {
    let (model, client) = makeModel()
    let yesterday = Self.today.adding(days: -1)
    client.push(
      .success(
        await userGoalScreenData(
          target: nil, history: [(iso(yesterday), 200, true)], today: (iso(Self.today), 0))),
      for: UserGoalScreen_Query.self)
    try await waitOnMain { model.hasData }
    #expect(model.state?.status == nil)
    #expect(model.hasGoal == false)
    #expect(model.state?.history.count == 1)
    #expect(model.state?.streaks.current == 1)
    #expect(model.state?.month?.rows.count ?? 0 >= 4)
    #expect(model.loadFailed == false)
  }

  @Test func withGoalDerivesStatusAndMergesToday() async throws {
    let (model, client) = makeModel()
    let yesterday = Self.today.adding(days: -1)
    client.push(
      .success(
        await userGoalScreenData(
          target: 300, history: [(iso(yesterday), 400, true), (iso(Self.today), 0, false)],
          today: (iso(Self.today), 120))),
      for: UserGoalScreen_Query.self)
    try await waitOnMain { model.state?.status != nil }
    let status = try #require(model.state?.status)
    #expect(status.additions == 120)
    #expect(status.remaining == 180)
    #expect(status.streak == 1)
    #expect(
      model.state?.history.last
        == UserGoalHistoryEntry(day: Self.today, target: 300, additions: 120, achieved: false))
    #expect(model.state?.streaks.current == 1)
  }

  @Test func emptyHistoryHasNoGrid() async throws {
    let (model, client) = makeModel()
    client.push(
      .success(await userGoalScreenData(target: nil, history: [], today: (iso(Self.today), 0))),
      for: UserGoalScreen_Query.self)
    try await waitOnMain { model.hasData }
    #expect(model.state?.month != nil)
    #expect(model.state?.streaks == UserGoalState.Streaks(current: 0, best: 0))
  }

  @Test func failureWithoutDataSetsLoadFailed() async throws {
    let (model, client) = makeModel()
    client.push(.failure(StubError(name: "load")), for: UserGoalScreen_Query.self)
    try await waitOnMain { model.loadFailed }
    #expect(model.hasData == false)
  }

  @Test func saveSendsTargetWithoutRefetch() async throws {
    let (model, client) = makeModel()
    client.push(
      .success(await userGoalScreenData(target: nil, history: [], today: (iso(Self.today), 0))),
      for: UserGoalScreen_Query.self)
    try await waitOnMain { model.hasData }
    client.stub(
      .success(await updatedUserGoalData(target: 500)),
      for: UserGoalScreen_UpdateUserGoal_Mutation.self)
    let before = client.refetchCount(of: UserGoalScreen_Query.self)
    #expect(await model.save(target: 500) == true)
    #expect(
      client.performed(UserGoalScreen_UpdateUserGoal_Mutation.self).last?.input
        .targetCharacterCount == 500)
    await drainMainActor()
    #expect(client.refetchCount(of: UserGoalScreen_Query.self) == before)
  }

  @Test func saveFailureReturnsFalseWithoutRefetch() async throws {
    let (model, client) = makeModel()
    client.stub(
      .failure(StubError(name: "save")), for: UserGoalScreen_UpdateUserGoal_Mutation.self)
    let before = client.refetchCount(of: UserGoalScreen_Query.self)
    #expect(await model.save(target: 500) == false)
    await drainMainActor()
    #expect(client.refetchCount(of: UserGoalScreen_Query.self) == before)
  }

  @Test func removePerformsMutationWithoutRefetch() async throws {
    let (model, client) = makeModel()
    client.stub(
      .success(await deletedUserGoalData()), for: UserGoalScreen_DeleteUserGoal_Mutation.self)
    let before = client.refetchCount(of: UserGoalScreen_Query.self)
    #expect(await model.remove() == true)
    #expect(client.performed(UserGoalScreen_DeleteUserGoal_Mutation.self).count == 1)
    await drainMainActor()
    #expect(client.refetchCount(of: UserGoalScreen_Query.self) == before)
  }

  @Test func removeFailureReturnsFalseWithoutRefetch() async throws {
    let (model, client) = makeModel()
    client.stub(
      .failure(StubError(name: "remove")), for: UserGoalScreen_DeleteUserGoal_Mutation.self)
    let before = client.refetchCount(of: UserGoalScreen_Query.self)
    #expect(await model.remove() == false)
    #expect(client.performed(UserGoalScreen_DeleteUserGoal_Mutation.self).count == 1)
    await drainMainActor()
    #expect(client.refetchCount(of: UserGoalScreen_Query.self) == before)
  }

  @Test func failureAfterDataStaysSilent() async throws {
    let (model, client) = makeModel()
    client.push(
      .success(
        await userGoalScreenData(
          target: 300, history: [(iso(Self.today.adding(days: -1)), 400, true)],
          today: (iso(Self.today), 120))),
      for: UserGoalScreen_Query.self)
    try await waitOnMain { model.hasData }
    client.push(.failure(StubError(name: "reload")), for: UserGoalScreen_Query.self)
    await drainMainActor()
    #expect(model.loadFailed == false)
    #expect(model.state != nil)
    #expect(model.state?.status?.additions == 120)
    #expect(model.hasData)
  }

  @Test func concurrentMutationIsRejected() async throws {
    let (model, client) = makeModel()
    client.stub(
      .success(await updatedUserGoalData(target: 500)),
      for: UserGoalScreen_UpdateUserGoal_Mutation.self)
    client.stub(
      .success(await deletedUserGoalData()), for: UserGoalScreen_DeleteUserGoal_Mutation.self)
    let gate = client.hold()
    let first = Task { await model.save(target: 500) }
    try await waitOnMain { model.isMutating }
    #expect(await model.remove() == false)
    #expect(await model.save(target: 900) == false)
    #expect(client.performedMutations.count == 1)
    await gate.open()
    #expect(await first.value == true)
    #expect(model.isMutating == false)
  }
}
