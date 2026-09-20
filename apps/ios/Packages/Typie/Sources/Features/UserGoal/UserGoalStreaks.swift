import Core

struct UserGoalHistoryEntry: Equatable, Sendable {
  let day: KSTDay
  let target: Int
  let additions: Int
  let achieved: Bool

  init(day: KSTDay, target: Int = 0, additions: Int, achieved: Bool) {
    self.day = day
    self.target = target
    self.additions = additions
    self.achieved = achieved
  }
}

struct UserGoalStatus: Equatable, Sendable {
  let additions: Int
  let target: Int
  let achieved: Bool
  let remaining: Int
  let streak: Int
  let bestStreak: Int

  init(
    additions: Int, target: Int, achieved: Bool, remaining: Int, streak: Int, bestStreak: Int
  ) {
    self.additions = additions
    self.target = target
    self.achieved = achieved
    self.remaining = remaining
    self.streak = streak
    self.bestStreak = bestStreak
  }
}

enum UserGoalStreaks {
  static func streaks(_ history: [UserGoalHistoryEntry], today: KSTDay) -> (
    current: Int, best: Int
  ) {
    let sorted = history.sorted { $0.day < $1.day }
    var best = 0
    var run = 0
    var previous: KSTDay?
    for entry in sorted {
      guard entry.achieved else {
        run = 0
        previous = nil
        continue
      }
      run = previous.map { $0.days(to: entry.day) == 1 } == true ? run + 1 : 1
      best = max(best, run)
      previous = entry.day
    }
    let achievedByDay = Dictionary(
      sorted.map { ($0.day, $0.achieved) }, uniquingKeysWith: { _, last in last })
    var cursor = achievedByDay[today] == true ? today : today.adding(days: -1)
    var current = 0
    while achievedByDay[cursor] == true {
      current += 1
      cursor = cursor.adding(days: -1)
    }
    return (current, best)
  }

  static func mergeToday(
    _ history: [UserGoalHistoryEntry], target: Int, todayAdditions: Int, today: KSTDay
  ) -> [UserGoalHistoryEntry] {
    history.filter { $0.day != today }
      + [
        UserGoalHistoryEntry(
          day: today, target: target, additions: todayAdditions,
          achieved: todayAdditions >= target)
      ]
  }

  static func status(
    history: [UserGoalHistoryEntry], target: Int, todayChange: (day: KSTDay, additions: Int),
    today: KSTDay
  ) -> UserGoalStatus {
    let additions = todayChange.day == today ? todayChange.additions : 0
    let live = mergeToday(history, target: target, todayAdditions: additions, today: today)
    let (current, best) = streaks(live, today: today)
    return UserGoalStatus(
      additions: additions, target: target, achieved: additions >= target,
      remaining: max(0, target - additions), streak: current, bestStreak: best)
  }
}
