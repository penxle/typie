import Core
import Foundation
import GraphQL

struct RecentDocument: Equatable, Sendable {
  let item: EntityRowItem
  let viewedAt: Date?

  var updatedAt: Date? {
    if case .document(let document) = item { return document.updatedAt }
    return nil
  }

  func date(for sort: RecentSort) -> Date? {
    switch sort {
    case .viewed: viewedAt
    case .updated: updatedAt
    }
  }

  static func make(_ document: RecentDocument_document) -> RecentDocument? {
    guard let item = EntityRowItem.make(document.entity.fragments.entityRow_entity, path: [])
    else { return nil }
    return RecentDocument(item: item, viewedAt: document.entity.viewedAt.flatMap(parseDateTime))
  }
}
