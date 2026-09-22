import Core
import Foundation
import GraphQL

enum HomeTreeNode: Equatable, Sendable, Identifiable {
  case folder(EntityRowItem, childCount: Int)
  case document(EntityRowItem)
  case divider(id: String)

  var id: String {
    switch self {
    case .folder(let item, _), .document(let item): item.entityId
    case .divider(let id): id
    }
  }

  var item: EntityRowItem? {
    switch self {
    case .folder(let item, _), .document(let item): item
    case .divider: nil
    }
  }

  static func make(_ entity: HomeTree_entity) -> HomeTreeNode? {
    if entity.node.__typename == "Divider" { return .divider(id: entity.id) }
    guard let item = EntityRowItem.make(entity.fragments.entityRow_entity, path: []) else {
      return nil
    }
    switch item {
    case .folder: return .folder(item, childCount: Int(entity.node.asFolder?.childCount ?? 0))
    case .document: return .document(item)
    }
  }
}
