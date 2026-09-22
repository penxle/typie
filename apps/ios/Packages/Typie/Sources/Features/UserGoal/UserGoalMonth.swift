import Core

struct UserGoalMonthCell: Equatable, Sendable {
  enum State: Equatable, Sendable {
    case achieved
    case missed
    case today
    case future
    case noGoal
    case out
  }

  let day: KSTDay
  let state: State
  let progress: Double

  init(day: KSTDay, state: State, progress: Double) {
    self.day = day
    self.state = state
    self.progress = progress
  }

  var isSelectable: Bool {
    switch state {
    case .future, .out: false
    default: true
    }
  }
}

struct UserGoalMonth: Equatable, Sendable {
  let year: Int
  let month: Int
  let rows: [[UserGoalMonthCell]]
  let today: KSTDay
  let todayColumn: Int

  init(history: [UserGoalHistoryEntry], today: KSTDay) {
    self.today = today
    year = today.year
    month = today.month
    todayColumn = today.weekday - 1
    let byDay = Dictionary(history.map { ($0.day, $0) }, uniquingKeysWith: { _, last in last })
    let first = KSTDay(year: today.year, month: today.month, day: 1)
    let start = first.adding(days: -(first.weekday - 1))
    var rows: [[UserGoalMonthCell]] = []
    var cursor = start
    repeat {
      var cells: [UserGoalMonthCell] = []
      for _ in 0..<7 {
        cells.append(Self.cell(cursor, today: today, byDay: byDay))
        cursor = cursor.adding(days: 1)
      }
      rows.append(cells)
    } while cursor.month == today.month && cursor.year == today.year
    self.rows = rows
  }

  private static func cell(
    _ day: KSTDay, today: KSTDay, byDay: [KSTDay: UserGoalHistoryEntry]
  ) -> UserGoalMonthCell {
    if day.month != today.month || day.year != today.year {
      return UserGoalMonthCell(day: day, state: .out, progress: 0)
    }
    if day > today {
      return UserGoalMonthCell(day: day, state: .future, progress: 0)
    }
    guard let entry = byDay[day], entry.target > 0 else {
      return UserGoalMonthCell(day: day, state: .noGoal, progress: 0)
    }
    let progress = min(1, Double(entry.additions) / Double(entry.target))
    if day == today {
      return UserGoalMonthCell(day: day, state: .today, progress: progress)
    }
    return UserGoalMonthCell(
      day: day, state: entry.achieved ? .achieved : .missed, progress: progress)
  }

  func row(of day: KSTDay) -> Int? {
    rows.firstIndex { $0.contains { $0.day == day } }
  }

  func cell(_ day: KSTDay) -> UserGoalMonthCell? {
    rows.joined().first { $0.day == day }
  }

  var monthLabel: String { "\(year)년 \(month)월" }

  func weekLabel(of day: KSTDay) -> String {
    guard day.month == month, day.year == year, let index = row(of: day) else { return monthLabel }
    return "\(monthLabel) \(index + 1)주차"
  }
}
