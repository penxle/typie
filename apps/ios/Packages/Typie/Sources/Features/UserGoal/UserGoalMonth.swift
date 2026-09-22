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
  let inMonth: Bool

  init(day: KSTDay, state: State, progress: Double, inMonth: Bool = true) {
    self.day = day
    self.state = state
    self.progress = progress
    self.inMonth = inMonth
  }

  var isSelectable: Bool {
    switch state {
    case .future, .out: false
    default: true
    }
  }
}

struct UserGoalMonth: Equatable, Sendable {
  nonisolated static let rowCount = 6

  let year: Int
  let month: Int
  let rows: [[UserGoalMonthCell]]
  let today: KSTDay
  let todayColumn: Int

  init(
    history: [UserGoalHistoryEntry], today: KSTDay, containing anchor: KSTDay? = nil,
    coverageStart: KSTDay? = nil
  ) {
    let anchor = anchor ?? today
    self.today = today
    year = anchor.year
    month = anchor.month
    todayColumn = today.weekday - 1
    let byDay = Dictionary(history.map { ($0.day, $0) }, uniquingKeysWith: { _, last in last })
    var cursor = Self.weekStart(of: KSTDay(year: anchor.year, month: anchor.month, day: 1))
    var rows: [[UserGoalMonthCell]] = []
    for _ in 0..<Self.rowCount {
      var cells: [UserGoalMonthCell] = []
      for _ in 0..<7 {
        cells.append(
          Self.cell(
            cursor, inMonth: cursor.month == anchor.month && cursor.year == anchor.year,
            today: today, coverageStart: coverageStart, byDay: byDay))
        cursor = cursor.adding(days: 1)
      }
      rows.append(cells)
    }
    self.rows = rows
  }

  static func weekLabel(for day: KSTDay, today: KSTDay) -> String {
    UserGoalMonth(history: [], today: today, containing: day).weekLabel(of: day)
  }

  static func lastDay(year: Int, month: Int) -> KSTDay {
    let next =
      month == 12
      ? KSTDay(year: year + 1, month: 1, day: 1) : KSTDay(year: year, month: month + 1, day: 1)
    return next.adding(days: -1)
  }

  static func weekStart(of day: KSTDay) -> KSTDay {
    day.adding(days: -(day.weekday - 1))
  }

  private static func cell(
    _ day: KSTDay, inMonth: Bool, today: KSTDay, coverageStart: KSTDay?,
    byDay: [KSTDay: UserGoalHistoryEntry]
  ) -> UserGoalMonthCell {
    if let coverageStart, day < coverageStart {
      return UserGoalMonthCell(day: day, state: .out, progress: 0, inMonth: inMonth)
    }
    if day > today {
      return UserGoalMonthCell(day: day, state: .future, progress: 0, inMonth: inMonth)
    }
    guard let entry = byDay[day], entry.target > 0 else {
      return UserGoalMonthCell(day: day, state: .noGoal, progress: 0, inMonth: inMonth)
    }
    let progress = min(1, Double(entry.additions) / Double(entry.target))
    if day == today {
      return UserGoalMonthCell(day: day, state: .today, progress: progress, inMonth: inMonth)
    }
    return UserGoalMonthCell(
      day: day, state: entry.achieved ? .achieved : .missed, progress: progress, inMonth: inMonth)
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
