import Core
import Foundation
import GraphQL

struct EntityDocumentItem: Equatable, Sendable {
  let entityId: String
  let icon: EntityIconSpec
  let path: [String]
  let title: String
  let subtitle: String?
  let excerpt: String
  let updatedAt: Date?
}

struct EntityFolderItem: Equatable, Sendable {
  let entityId: String
  let icon: EntityIconSpec
  let path: [String]
  let title: String
  let folderCount: Int
  let documentCount: Int

  var summary: String {
    EntityText.folderSummary(folders: folderCount, documents: documentCount)
  }
}

enum EntityRowItem: Equatable, Sendable, Identifiable {
  case document(EntityDocumentItem)
  case folder(EntityFolderItem)

  var entityId: String {
    switch self {
    case .document(let item): item.entityId
    case .folder(let item): item.entityId
    }
  }

  var id: String { entityId }

  var icon: EntityIconSpec {
    switch self {
    case .document(let item): item.icon
    case .folder(let item): item.icon
    }
  }

  var path: [String] {
    switch self {
    case .document(let item): item.path
    case .folder(let item): item.path
    }
  }

  var title: String {
    switch self {
    case .document(let item): item.title
    case .folder(let item): item.title
    }
  }

  func trailing(now: Date) -> String? {
    switch self {
    case .document(let item): item.updatedAt.map { timeAgo($0, now: now) }
    case .folder(let item): item.summary
    }
  }
}

extension EntityRowItem {
  static func make(_ row: EntityRow_entity, path: EntityRowPath_entity) -> EntityRowItem? {
    make(row, path: Self.path(path))
  }

  static func make(_ row: EntityRow_entity, path segments: [String]) -> EntityRowItem? {
    let icon = row.fragments.entityIcon_entity
    if let document = row.node.asDocument?.fragments.entityRowDocument_document {
      return .document(
        EntityDocumentItem(
          entityId: row.id, icon: iconSpec(icon, kind: .document), path: segments,
          title: EntityText.documentTitle(document.title), subtitle: document.subtitle,
          excerpt: document.excerpt, updatedAt: parseDateTime(document.updatedAt)))
    }
    if let folder = row.node.asFolder?.fragments.entityRowFolder_folder {
      return .folder(
        EntityFolderItem(
          entityId: row.id, icon: iconSpec(icon, kind: .folder), path: segments,
          title: EntityText.folderName(folder.name), folderCount: folder.folderCount,
          documentCount: folder.documentCount))
    }
    return nil
  }

  static func iconSpec(_ icon: EntityIcon_entity, kind: EntityKind) -> EntityIconSpec {
    EntityIconSpec(kind: kind, name: icon.icon, color: icon.iconColor)
  }

  static func path(_ fragment: EntityRowPath_entity) -> [String] {
    fragment.ancestors.compactMap { $0.node.asFolder?.name }.map(EntityText.folderName)
  }
}
