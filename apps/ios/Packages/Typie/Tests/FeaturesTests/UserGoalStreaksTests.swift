import Core
import Testing

@testable import Features

@Suite struct UserGoalStreaksTests {
  private let today = KSTDay(year: 2026, month: 9, day: 21)

  private func entry(_ daysAgo: Int, _ additions: Int, achieved: Bool) -> UserGoalHistoryEntry {
    UserGoalHistoryEntry(
      day: today.adding(days: -daysAgo), additions: additions, achieved: achieved)
  }

  @Test func countsRunIncludingToday() {
    let history = [
      entry(2, 500, achieved: true), entry(1, 500, achieved: true), entry(0, 500, achieved: true),
    ]
    let result = UserGoalStreaks.streaks(history, today: today)
    #expect(result.current == 3)
    #expect(result.best == 3)
  }

  @Test func todayMissDoesNotBreakRun() {
    let history = [
      entry(2, 500, achieved: true), entry(1, 500, achieved: true), entry(0, 10, achieved: false),
    ]
    #expect(UserGoalStreaks.streaks(history, today: today).current == 2)
  }

  @Test func missedDayBreaksRun() {
    let history = [
      entry(3, 500, achieved: true), entry(2, 0, achieved: false), entry(1, 500, achieved: true),
      entry(0, 500, achieved: true),
    ]
    let result = UserGoalStreaks.streaks(history, today: today)
    #expect(result.current == 2)
    #expect(result.best == 2)
  }

  @Test func dayWithoutRowBreaksRun() {
    let history = [
      entry(3, 500, achieved: true), entry(1, 500, achieved: true), entry(0, 500, achieved: true),
    ]
    let result = UserGoalStreaks.streaks(history, today: today)
    #expect(result.current == 2)
    #expect(result.best == 2)
  }

  @Test func bestComesFromPastRun() {
    let history = [
      entry(6, 1, achieved: true), entry(5, 1, achieved: true), entry(4, 1, achieved: true),
      entry(3, 0, achieved: false), entry(0, 1, achieved: true),
    ]
    let result = UserGoalStreaks.streaks(history, today: today)
    #expect(result.current == 1)
    #expect(result.best == 3)
  }

  @Test func emptyHistoryIsZero() {
    let result = UserGoalStreaks.streaks([], today: today)
    #expect(result.current == 0)
    #expect(result.best == 0)
  }

  @Test func bestIgnoresInputOrder() {
    let ordered = [
      entry(2, 1, achieved: true), entry(1, 1, achieved: true), entry(0, 1, achieved: true),
    ]
    let shuffled = [ordered[2], ordered[0], ordered[1]]
    #expect(UserGoalStreaks.streaks(shuffled, today: today).best == 3)
  }

  @Test func mergeTodayReplacesStaleTodayRow() {
    let history = [entry(1, 500, achieved: true), entry(0, 500, achieved: true)]
    let merged = UserGoalStreaks.mergeToday(history, target: 300, todayAdditions: 100, today: today)
    #expect(merged.count == 2)
    #expect(
      merged.last
        == UserGoalHistoryEntry(day: today, target: 300, additions: 100, achieved: false))
  }

  @Test func statusUsesTodayChangeAndComputesRemaining() {
    let history = [entry(1, 500, achieved: true), entry(0, 0, achieved: false)]
    let status = UserGoalStreaks.status(
      history: history, target: 300, todayChange: (day: today, additions: 120), today: today)
    #expect(
      status
        == UserGoalStatus(
          additions: 120, target: 300, achieved: false, remaining: 180, streak: 1, bestStreak: 1))
  }

  @Test func statusIgnoresTodayChangeFromAnotherDay() {
    let status = UserGoalStreaks.status(
      history: [], target: 300, todayChange: (day: today.adding(days: -1), additions: 900),
      today: today)
    #expect(status.additions == 0)
    #expect(status.remaining == 300)
  }

  @Test func statusAtExactTargetIsAchievedWithZeroRemaining() {
    let status = UserGoalStreaks.status(
      history: [], target: 300, todayChange: (day: today, additions: 300), today: today)
    #expect(status.achieved)
    #expect(status.remaining == 0)
    #expect(status.streak == 1)
  }
}
