import Core
import GraphQL

struct UserGoalDayDocument: Equatable, Sendable, Identifiable {
  let id: String
  let entityId: String
  let title: String
  let additions: Int

  init(id: String, entityId: String, title: String, additions: Int) {
    self.id = id
    self.entityId = entityId
    self.title = title
    self.additions = additions
  }
}

extension UserGoalDayDocument {
  init(_ row: UserGoalDay_Query.Data.Me.DailyDocumentCharacterCountChange) {
    self.init(
      id: row.document.id, entityId: row.document.entity.id, title: row.document.title,
      additions: row.additions)
  }
}
