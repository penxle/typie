import Core
import Foundation
import GraphQL

enum EntityContainerItem: Equatable, Sendable, Identifiable {
  case entity(EntityRowItem)
  case divider(id: String)

  var id: String {
    switch self {
    case .entity(let item): item.entityId
    case .divider(let id): id
    }
  }

  var entity: EntityRowItem? {
    switch self {
    case .entity(let item): item
    case .divider: nil
    }
  }

  static func make(_ row: EntityRow_entity) -> EntityContainerItem? {
    if row.node.__typename == "Divider" { return .divider(id: row.id) }
    return EntityRowItem.make(row, path: []).map(EntityContainerItem.entity)
  }

  static let placeholderRows: [EntityContainerItem] = (1...3).map { index in
    .entity(
      .document(
        EntityDocumentItem(
          entityId: "placeholder-\(index)",
          icon: EntityIconSpec(kind: .document, name: "", color: ""), path: [],
          title: EntityText.documentTitle(""), subtitle: nil, excerpt: "", updatedAt: nil)))
  }
}

struct EntityContainerHeroState: Equatable, Sendable {
  let icon: EntityIconSpec?
  let title: String
  let summary: String
}
