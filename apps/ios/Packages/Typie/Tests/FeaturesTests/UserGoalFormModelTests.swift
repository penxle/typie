import Core
import FactoryKit
import FactoryTesting
import Foundation
import GraphQL
import Testing

@testable import Design
@testable import Features

@Suite struct UserGoalFormModelTests {
  @Test func parsesPositiveIntegersWithinInt32() {
    #expect(UserGoalFormModel.parse("300") == 300)
    #expect(UserGoalFormModel.parse(" 1,000 ") == 1000)
    #expect(UserGoalFormModel.parse("2147483647") == 2_147_483_647)
  }

  @Test func rejectsBlankZeroNegativeNonNumericAndOverflow() {
    #expect(UserGoalFormModel.parse("") == nil)
    #expect(UserGoalFormModel.parse("0") == nil)
    #expect(UserGoalFormModel.parse("-5") == nil)
    #expect(UserGoalFormModel.parse("abc") == nil)
    #expect(UserGoalFormModel.parse("12a") == nil)
    #expect(UserGoalFormModel.parse("2147483648") == nil)
  }

  @Test func ruleReportsInvalidMessage() {
    #expect(UserGoalFormModel.rule.validate("0") == UserGoalFormModel.invalidMessage)
    #expect(UserGoalFormModel.rule.validate("42") == nil)
  }
}

@MainActor
@Suite(.container) struct UserGoalFormModelSeedingTests {
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

  private func load(_ client: FakeGraphQLClient, target: Int?) async {
    client.push(
      .success(
        await userGoalScreenData(target: target, history: [], today: (iso(Self.today), 0))),
      for: UserGoalScreen_Query.self)
  }

  @Test func seedsTargetFromExistingGoal() async throws {
    let (model, client) = makeModel()
    await load(client, target: 300)
    try await waitOnMain { model.hasData }
    let form = UserGoalFormModel(goal: model)
    #expect(form.hasGoal)
    #expect(form.target.value == "300")
  }

  @Test func seedsEmptyTargetWithoutGoal() async throws {
    let (model, client) = makeModel()
    await load(client, target: nil)
    try await waitOnMain { model.hasData }
    let form = UserGoalFormModel(goal: model)
    #expect(form.hasGoal == false)
    #expect(form.target.value == "")
  }

  @Test func submitRejectsReentrance() async throws {
    let (model, client) = makeModel()
    await load(client, target: 300)
    try await waitOnMain { model.hasData }
    client.stub(
      .success(await updatedUserGoalData(target: 300)),
      for: UserGoalScreen_UpdateUserGoal_Mutation.self)
    let form = UserGoalFormModel(goal: model)
    let gate = client.hold()
    let first = Task { await form.submit() }
    try await waitOnMain { form.isSubmitting }
    #expect(await form.submit() == nil)
    #expect(client.performed(UserGoalScreen_UpdateUserGoal_Mutation.self).count == 1)
    await gate.open()
    #expect(await first.value == true)
    #expect(form.isSubmitting == false)
  }
}
