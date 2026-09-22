import Core
import Testing

@testable import Features

@Suite struct UserGoalStateTests {
  private let today = KSTDay(year: 2026, month: 9, day: 21)
  private let todayISO = "2026-09-20T15:00:00.000Z"
  private let yesterdayISO = "2026-09-19T15:00:00.000Z"

  @Test func withGoalDerivesStatusMergedHistoryAndGrid() async {
    let snapshot = UserGoalSnapshot(
      await goalUser(
        target: 300, history: [(yesterdayISO, 400, true), (todayISO, 0, false)],
        today: (todayISO, 120)))
    let state = UserGoalState(snapshot: snapshot, today: today)
    #expect(state.status?.additions == 120)
    #expect(state.status?.remaining == 180)
    #expect(
      state.history.last
        == UserGoalHistoryEntry(day: today, target: 300, additions: 120, achieved: false))
    #expect(state.streaks == UserGoalState.Streaks(current: 1, best: 1))
    #expect(state.hasGoal)
  }

  @Test func withoutGoalKeepsRawHistoryAndStreaks() async {
    let snapshot = UserGoalSnapshot(
      await goalUser(target: nil, history: [(yesterdayISO, 200, true)], today: (todayISO, 0)))
    let state = UserGoalState(snapshot: snapshot, today: today)
    #expect(state.status == nil)
    #expect(state.hasGoal == false)
    #expect(state.history.count == 1)
    #expect(state.streaks.current == 1)
  }

  @Test func emptyHistoryHasNoGrid() async {
    let state = UserGoalState(
      snapshot: UserGoalSnapshot(await goalUser(target: nil, history: [], today: (todayISO, 0))),
      today: today)
    #expect(state.streaks == UserGoalState.Streaks(current: 0, best: 0))
  }

  @Test func placeholderHasShapeButNoFacts() {
    let state = UserGoalState.placeholder
    #expect(state.hasGoal == false)
    #expect(state.status == nil)
    #expect(state.streaks == UserGoalState.Streaks(current: 0, best: 0))
    #expect(state.history.isEmpty)
  }

  @Test func missingTodayChangeCountsAsZero() async {
    let state = UserGoalState(
      snapshot: UserGoalSnapshot(
        await goalUser(target: 300, history: [], today: ("not-a-date", 900))), today: today)
    #expect(state.status?.additions == 0)
    #expect(state.status?.remaining == 300)
  }
}
