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

  @Test func listsWeeksAndMonthPagesAndKeepsTheSelectionWhilePaging() async throws {
    let client = FakeGraphQLClient()
    registerFake(client)
    let model = Container.shared.userGoalModel()
    model.now = { Self.now }
    client.push(
      .success(
        await userGoalScreenData(
          target: 300, history: [(iso(Self.today), 0, false)], today: (iso(Self.today), 0))),
      for: UserGoalScreen_Query.self)
    try await waitOnMain { model.hasData }
    let windowStart = Self.today.adding(days: -364)
    let thisWeek = UserGoalMonth.weekStart(of: Self.today)
    let weeks = model.weeks
    #expect(weeks.count == 52 || weeks.count == 53)
    #expect(weeks.first == UserGoalMonth.weekStart(of: windowStart))
    #expect(weeks.last == thisWeek)
    #expect(model.displayedWeek == thisWeek)
    let pages = model.monthPages
    #expect(pages.count == 12 || pages.count == 13)
    #expect(pages.contains(thisWeek))
    #expect(pages == pages.sorted())
    #expect(Set(pages.map { "\($0.year)-\($0.month)" }).count == pages.count)
    #expect(pages.allSatisfy { weeks.contains($0) })
    model.show(week: weeks[weeks.count - 2])
    #expect(model.displayedWeek == weeks[weeks.count - 2])
    #expect(model.anchor == weeks[weeks.count - 2])
    #expect(model.selectedDay == Self.today)
    model.select(Self.today.adding(days: -1))
    #expect(model.anchor == Self.today.adding(days: -1))
    #expect(model.displayedWeek == weeks[weeks.count - 2])
    model.settleDisplayedWeek()
    #expect(model.displayedWeek == UserGoalMonth.weekStart(of: Self.today.adding(days: -1)))
    let firstOfMonth = KSTDay(year: Self.today.year, month: Self.today.month, day: 1)
    model.select(firstOfMonth)
    model.settleDisplayedWeek()
    #expect(model.displayedWeek == UserGoalMonth.weekStart(of: firstOfMonth))
    #expect(model.grid(for: model.displayedWeek)?.month == firstOfMonth.month)
    #expect(
      model.grid(for: model.displayedWeek)?.weekLabel(of: model.anchor).hasSuffix("1주차") == true)
    #expect(model.monthPages.contains(model.displayedWeek))
    let beforeWindow = model.grid(for: weeks[0])?.cell(windowStart.adding(days: -1))
    #expect(beforeWindow == nil || beforeWindow?.state == .out)
    #expect(model.grid(for: weeks[0])?.cell(windowStart)?.state != .out)
  }

  @Test func noGoalDayShowsThatDaysCharacterCountImmediately() async throws {
    let client = FakeGraphQLClient()
    registerFake(client)
    let model = Container.shared.userGoalModel()
    model.now = { Self.now }
    let yesterday = Self.today.adding(days: -1)
    let before = Self.today.adding(days: -2)
    client.push(
      .success(
        await userGoalScreenData(
          target: 300, history: [(iso(yesterday), 400, true), (iso(Self.today), 0, false)],
          today: (iso(Self.today), 0), changes: [(iso(before), 300), (iso(yesterday), 400)])),
      for: UserGoalScreen_Query.self)
    try await waitOnMain { model.hasData }
    model.select(before)
    #expect(model.selected?.hasGoal == false)
    #expect(model.selected?.countText == "300자")
    #expect(model.selected?.sentence == "목표가 없던 날이에요")
    model.select(yesterday)
    #expect(model.selected?.hasGoal == true)
    #expect(model.selected?.countText == "400자")
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
