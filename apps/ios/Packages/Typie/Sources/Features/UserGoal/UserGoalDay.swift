import Core

struct UserGoalDay: Equatable, Sendable {
  let day: KSTDay
  let isToday: Bool
  let target: Int?
  let additions: Int
  let achieved: Bool

  init(day: KSTDay, isToday: Bool, target: Int?, additions: Int, achieved: Bool) {
    self.day = day
    self.isToday = isToday
    self.target = target
    self.additions = additions
    self.achieved = achieved
  }

  init(day: KSTDay, today: KSTDay, history: [UserGoalHistoryEntry]) {
    let entry = history.last { $0.day == day }
    self.init(
      day: day, isToday: day == today, target: entry.map(\.target).flatMap { $0 > 0 ? $0 : nil },
      additions: entry?.additions ?? 0, achieved: entry?.achieved ?? false)
  }

  var hasGoal: Bool { target != nil }

  var progress: Double {
    guard let target, target > 0 else { return 0 }
    return min(1, Double(additions) / Double(target))
  }

  var percent: Int {
    guard let target, target > 0 else { return 0 }
    return Int((Double(additions) / Double(target) * 100).rounded())
  }

  var percentText: String {
    percent > 999 ? "999%+" : "\(percent)%"
  }

  var title: String {
    isToday ? "오늘" : UserGoalFormat.dayLabel(day)
  }

  var countText: String { UserGoalFormat.characters(additions) }

  var sentence: String {
    guard let target else { return isToday ? "설정된 목표가 없어요" : "목표가 없던 날이에요" }
    if achieved { return "목표 \(UserGoalFormat.characters(target))를 달성했어요" }
    if isToday {
      if additions == 0 { return "아직 쓰기 전이에요" }
      if additions * 2 >= target {
        return
          "\(UserGoalFormat.characters(target))까지 \(UserGoalFormat.characters(target - additions)) 남았어요"
      }
    }
    return "목표 \(UserGoalFormat.characters(target))"
  }

  var accessibilityLabel: String {
    "\(title), \(countText), \(sentence)"
  }
}
