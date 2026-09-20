import Core
import Testing

@testable import Features

@Suite struct UserGoalMonthTests {
  private let today = KSTDay(year: 2026, month: 9, day: 21)

  private func entry(_ day: Int, target: Int = 1000, additions: Int, achieved: Bool)
    -> UserGoalHistoryEntry
  {
    UserGoalHistoryEntry(
      day: KSTDay(year: 2026, month: 9, day: day), target: target, additions: additions,
      achieved: achieved)
  }

  @Test func laysOutSeptember2026InFiveRowsWithOutOfMonthCells() {
    let month = UserGoalMonth(history: [], today: today)
    #expect(month.rows.count == 5)
    #expect(month.rows.allSatisfy { $0.count == 7 })
    #expect(month.rows[0][0].day == KSTDay(year: 2026, month: 8, day: 30))
    #expect(month.rows[0][0].state == .out)
    #expect(month.rows[0][2].day == KSTDay(year: 2026, month: 9, day: 1))
    #expect(month.rows[4][3].day == KSTDay(year: 2026, month: 9, day: 30))
    #expect(month.rows[4][4].state == .out)
    #expect(month.todayColumn == 1)
    #expect(month.monthLabel == "2026년 9월")
  }

  @Test func mapsStatesFromHistoryAndToday() {
    let history = [
      entry(20, additions: 1803, achieved: true),
      entry(16, additions: 546, achieved: false),
      entry(21, additions: 640, achieved: false),
    ]
    let month = UserGoalMonth(history: history, today: today)
    #expect(month.cell(KSTDay(year: 2026, month: 9, day: 20))?.state == .achieved)
    #expect(month.cell(KSTDay(year: 2026, month: 9, day: 16))?.state == .missed)
    #expect(month.cell(today)?.state == .today)
    #expect(month.cell(today)?.progress == 0.64)
    #expect(month.cell(KSTDay(year: 2026, month: 9, day: 22))?.state == .future)
    #expect(month.cell(KSTDay(year: 2026, month: 9, day: 1))?.state == .noGoal)
    #expect(month.cell(KSTDay(year: 2026, month: 9, day: 22))?.isSelectable == false)
    #expect(month.cell(KSTDay(year: 2026, month: 9, day: 1))?.isSelectable == true)
  }

  @Test func weekLabelCountsRowsFromTheOneHoldingTheFirst() {
    let month = UserGoalMonth(history: [], today: today)
    #expect(month.row(of: today) == 3)
    #expect(month.weekLabel(of: today) == "2026년 9월 4주차")
    #expect(month.weekLabel(of: KSTDay(year: 2026, month: 9, day: 1)) == "2026년 9월 1주차")
    #expect(month.weekLabel(of: KSTDay(year: 2026, month: 10, day: 1)) == "2026년 9월")
  }

  @Test func sixRowMonthWhenFirstFallsLateInWeek() {
    let month = UserGoalMonth(history: [], today: KSTDay(year: 2026, month: 8, day: 30))
    #expect(month.rows.count == 6)
    #expect(month.rows[5][0].day == KSTDay(year: 2026, month: 8, day: 30))
  }
}
