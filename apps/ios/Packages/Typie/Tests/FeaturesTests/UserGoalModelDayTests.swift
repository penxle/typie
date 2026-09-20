import Core
import FactoryKit
import FactoryTesting
import Foundation
import GraphQL
import Testing

@testable import Features

@MainActor
@Suite(.container) struct UserGoalModelDayTests {
  private static let now = Date(timeIntervalSince1970: 1_790_000_000)
  private static let today = KSTDay(now)

  private func makeModel() async throws -> (UserGoalModel, FakeGraphQLClient) {
    let client = FakeGraphQLClient()
    registerFake(client)
    let model = Container.shared.userGoalModel()
    model.now = { Self.now }
    let yesterday = Self.today.adding(days: -1)
    client.push(
      .success(
        await userGoalScreenData(
          target: 300, history: [(iso(yesterday), 400, true), (iso(Self.today), 0, false)],
          today: (iso(Self.today), 120))),
      for: UserGoalScreen_Query.self)
    try await waitOnMain { model.hasData }
    return (model, client)
  }

  private func iso(_ day: KSTDay) -> String {
    ISO8601DateFormatter().string(from: day.date)
  }

  @Test func selectsTodayByDefaultAndReadsItsDocuments() async throws {
    let (model, client) = try await makeModel()
    #expect(model.selectedDay == Self.today)
    #expect(model.selected?.isToday == true)
    #expect(model.selected?.countText == "120자")
    #expect(model.selected?.sentence == "목표 300자")
    client.push(
      .success(await userGoalDayData([("doc-1", "겨울의 문장들", 80), ("doc-2", "일기", 40)])),
      for: UserGoalDay_Query.self)
    try await waitOnMain { model.documents.count == 2 }
    #expect(model.documents.first?.title == "겨울의 문장들")
    #expect(model.documents.first?.entityId == "doc-1-entity")
    #expect(model.documentsFailed == false)
  }

  @Test func selectingAPastDayDerivesThatDayAndRefetchesDocuments() async throws {
    let (model, client) = try await makeModel()
    let yesterday = Self.today.adding(days: -1)
    model.select(yesterday)
    #expect(model.selectedDay == yesterday)
    #expect(model.selected?.isToday == false)
    #expect(model.selected?.achieved == true)
    #expect(model.selected?.sentence == "목표 300자를 달성했어요")
    client.push(
      .success(await userGoalDayData([("doc-9", "에세이 초고", 400)])), for: UserGoalDay_Query.self)
    try await waitOnMain { model.documents.count == 1 }
    #expect(model.documents.first?.additions == 400)
  }

  @Test func ignoresFutureDaysAndReportsDocumentFailure() async throws {
    let (model, client) = try await makeModel()
    model.select(Self.today.adding(days: 1))
    #expect(model.selectedDay == Self.today)
    client.push(.failure(StubError(name: "day")), for: UserGoalDay_Query.self)
    try await waitOnMain { model.documentsFailed }
    #expect(model.documents.isEmpty)
  }
}
