import Core
import Foundation
import GraphQL

struct RecentDocument: Equatable, Sendable {
  let item: EntityRowItem
  let viewedAt: Date?

  static func make(_ document: RecentDocument_document) -> RecentDocument? {
    guard let item = EntityRowItem.make(document.entity.fragments.entityRow_entity, path: [])
    else { return nil }
    return RecentDocument(item: item, viewedAt: document.entity.viewedAt.flatMap(parseDateTime))
  }
}
