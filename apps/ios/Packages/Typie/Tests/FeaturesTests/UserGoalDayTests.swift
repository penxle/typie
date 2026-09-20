import Core
import Testing

@testable import Features

@Suite struct UserGoalDayTests {
  private let today = KSTDay(year: 2026, month: 9, day: 21)

  private func day(
    _ offset: Int, target: Int?, additions: Int, achieved: Bool
  ) -> UserGoalDay {
    UserGoalDay(
      day: today.adding(days: offset), isToday: offset == 0, target: target, additions: additions,
      achieved: achieved)
  }

  @Test func percentRoundsAndCaps() {
    #expect(day(0, target: 1000, additions: 640, achieved: false).percentText == "64%")
    #expect(day(0, target: 1000, additions: 1280, achieved: true).percentText == "128%")
    #expect(day(0, target: 100, additions: 100_000, achieved: true).percentText == "999%+")
    #expect(day(0, target: 1000, additions: 0, achieved: false).progress == 0)
    #expect(day(0, target: 1000, additions: 1280, achieved: true).progress == 1)
  }

  @Test func todayUsesToGoFramingPastHalf() {
    #expect(day(0, target: 1000, additions: 640, achieved: false).sentence == "1,000자까지 360자 남았어요")
    #expect(day(0, target: 1000, additions: 300, achieved: false).sentence == "목표 1,000자")
    #expect(day(0, target: 1000, additions: 0, achieved: false).sentence == "아직 쓰기 전이에요")
    #expect(day(0, target: 1000, additions: 1280, achieved: true).sentence == "목표 1,000자를 달성했어요")
  }

  @Test func pastDaysStateFactsWithoutJudgement() {
    #expect(day(-5, target: 800, additions: 546, achieved: false).sentence == "목표 800자")
    #expect(day(-1, target: 1000, additions: 1803, achieved: true).sentence == "목표 1,000자를 달성했어요")
    #expect(day(-1, target: 1000, additions: 1803, achieved: true).title == "9월 20일 일")
    #expect(day(0, target: 1000, additions: 1, achieved: false).title == "오늘")
  }

  @Test func noGoalDayHasNoRingAndOptionalCount() {
    let empty = day(-20, target: nil, additions: 0, achieved: false)
    #expect(empty.hasGoal == false)
    #expect(empty.sentence == "목표가 없던 날이에요")
    #expect(empty.noGoalDetail == nil)
    let written = day(-20, target: nil, additions: 300, achieved: false)
    #expect(written.noGoalDetail == "300자를 썼어요")
    #expect(written.accessibilityLabel == "9월 1일 화, 목표가 없던 날이에요, 300자를 썼어요")
  }

  @Test func derivesFromHistoryUsingThatDaysTarget() {
    let history = [
      UserGoalHistoryEntry(
        day: today.adding(days: -12), target: 800, additions: 900, achieved: true),
      UserGoalHistoryEntry(day: today, target: 1000, additions: 640, achieved: false),
    ]
    let past = UserGoalDay(day: today.adding(days: -12), today: today, history: history)
    #expect(past.target == 800)
    #expect(past.percent == 113)
    #expect(past.sentence == "목표 800자를 달성했어요")
    let none = UserGoalDay(day: today.adding(days: -3), today: today, history: history)
    #expect(none.hasGoal == false)
  }
}
