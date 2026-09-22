import Core
import Testing

@testable import Features

@Suite struct UserGoalSnapshotTests {
  @Test func mapsFieldsIntoKoreanDays() async {
    let snapshot = UserGoalSnapshot(
      await goalUser(
        target: 300,
        history: [
          ("2026-09-19T15:00:00.000Z", 400, true), ("2026-09-20T15:00:00.000Z", 10, false),
        ],
        today: ("2026-09-20T15:00:00.000Z", 50)))
    #expect(snapshot.target == 300)
    #expect(
      snapshot.history == [
        UserGoalHistoryEntry(
          day: KSTDay(year: 2026, month: 9, day: 20), target: 300, additions: 400, achieved: true),
        UserGoalHistoryEntry(
          day: KSTDay(year: 2026, month: 9, day: 21), target: 300, additions: 10, achieved: false),
      ])
    #expect(
      snapshot.todayChange
        == UserGoalSnapshot.TodayChange(
          day: KSTDay(year: 2026, month: 9, day: 21), additions: 50))
  }

  @Test func nilGoalAndUnparsableRowsAreDropped() async {
    let snapshot = UserGoalSnapshot(
      await goalUser(
        target: nil, history: [("not-a-date", 1, true)], today: ("not-a-date", 7)))
    #expect(snapshot.target == nil)
    #expect(snapshot.history.isEmpty)
    #expect(snapshot.todayChange == nil)
  }

  @Test func parsesTimestampsWithoutFractionalSeconds() async {
    let snapshot = UserGoalSnapshot(
      await goalUser(target: 100, history: [], today: ("2026-09-20T15:00:00Z", 20)))
    #expect(snapshot.todayChange?.day == KSTDay(year: 2026, month: 9, day: 21))
  }
}
