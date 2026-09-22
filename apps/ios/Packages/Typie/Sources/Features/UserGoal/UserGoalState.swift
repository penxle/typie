import Core

struct UserGoalState: Equatable, Sendable {
  struct Streaks: Equatable, Sendable {
    let current: Int
    let best: Int

    init(current: Int, best: Int) {
      self.current = current
      self.best = best
    }
  }

  let status: UserGoalStatus?
  let history: [UserGoalHistoryEntry]
  let streaks: Streaks
  let month: UserGoalMonth?

  var hasGoal: Bool { status != nil }

  static let placeholder = UserGoalState(
    status: nil, history: [], streaks: Streaks(current: 0, best: 0), month: nil)

  init(
    status: UserGoalStatus?, history: [UserGoalHistoryEntry], streaks: Streaks,
    month: UserGoalMonth?
  ) {
    self.status = status
    self.history = history
    self.streaks = streaks
    self.month = month
  }

  init(snapshot: UserGoalSnapshot, today: KSTDay) {
    let todayChange =
      snapshot.todayChange.map { (day: $0.day, additions: $0.additions) }
      ?? (day: today, additions: 0)
    if let target = snapshot.target {
      let status = UserGoalStreaks.status(
        history: snapshot.history, target: target, todayChange: todayChange, today: today)
      self.status = status
      history = UserGoalStreaks.mergeToday(
        snapshot.history, target: target, todayAdditions: status.additions, today: today)
      streaks = Streaks(current: status.streak, best: status.bestStreak)
    } else {
      status = nil
      history = snapshot.history
      let raw = UserGoalStreaks.streaks(snapshot.history, today: today)
      streaks = Streaks(current: raw.current, best: raw.best)
    }
    month = UserGoalMonth(history: history, today: today)
  }
}
