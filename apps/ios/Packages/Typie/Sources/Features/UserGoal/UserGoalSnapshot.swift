import Core
import Foundation
import GraphQL

struct UserGoalSnapshot: Equatable, Sendable {
  struct TodayChange: Equatable, Sendable {
    let day: KSTDay
    let additions: Int

    init(day: KSTDay, additions: Int) {
      self.day = day
      self.additions = additions
    }
  }

  let target: Int?
  let history: [UserGoalHistoryEntry]
  let todayChange: TodayChange?

  init(_ user: UserGoalSection_user) {
    target = user.goal?.targetCharacterCount
    history = user.goalHistory.compactMap { row in
      parseDateTime(row.date).map {
        UserGoalHistoryEntry(
          day: KSTDay($0), target: row.targetCharacterCount, additions: row.additions,
          achieved: row.achieved)
      }
    }
    let today = user.todayCharacterCountChange
    todayChange = parseDateTime(today.date).map {
      TodayChange(day: KSTDay($0), additions: today.additions)
    }
  }
}
