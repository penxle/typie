import Core
import Foundation

public struct SearchDocumentHit: Equatable, Sendable {
  public let entityId: String
  public let icon: EntityIconSpec
  public let path: [String]
  public let title: HighlightedText
  public let subtitle: HighlightedText?
  public let preview: HighlightedText
  public let previewLines: Int
  public let updatedAt: Date?
}

public struct SearchFolderHit: Equatable, Sendable {
  public let entityId: String
  public let icon: EntityIconSpec
  public let path: [String]
  public let title: HighlightedText
  public let summary: String
}

public enum SearchHit: Equatable, Sendable, Identifiable {
  case document(SearchDocumentHit)
  case folder(SearchFolderHit)

  public var entityId: String {
    switch self {
    case .document(let hit): hit.entityId
    case .folder(let hit): hit.entityId
    }
  }

  public var id: String { entityId }
}

extension SearchHit {
  static func make(from hits: [SearchScreen_Search_Query.Data.Search.Hit]) -> [SearchHit] {
    hits.compactMap { hit in
      if let document = hit.asSearchHitDocument?.fragments.searchResultDocument_hit {
        return Self.document(document).map(SearchHit.document)
      }
      if let folder = hit.asSearchHitFolder?.fragments.searchResultFolder_hit {
        return Self.folder(folder).map(SearchHit.folder)
      }
      return nil
    }
  }

  private static func document(_ hit: SearchResultDocument_hit) -> SearchDocumentHit? {
    let entity = hit.document.entity
    let row = entity.fragments.entityRow_entity
    guard let document = row.node.asDocument?.fragments.entityRowDocument_document else {
      return nil
    }
    let subtitle =
      hit.subtitle.map(HighlightedText.init(markup:))
      ?? document.subtitle.map(HighlightedText.init(plain:))
    return SearchDocumentHit(
      entityId: row.id,
      icon: iconSpec(row.fragments.entityIcon_entity, kind: .document),
      path: path(entity.fragments.entityRowPath_entity),
      title: hit.title.map(HighlightedText.init(markup:))
        ?? HighlightedText(plain: EntityText.documentTitle(document.title)),
      subtitle: subtitle.flatMap {
        $0.plain.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty ? nil : $0
      },
      preview: hit.text.map(HighlightedText.init(markup:))
        ?? HighlightedText(plain: EntityText.excerpt(document.excerpt)),
      previewLines: hit.text == nil ? 1 : 2,
      updatedAt: parseDateTime(document.updatedAt))
  }

  private static func folder(_ hit: SearchResultFolder_hit) -> SearchFolderHit? {
    let entity = hit.folder.entity
    let row = entity.fragments.entityRow_entity
    guard let folder = row.node.asFolder?.fragments.entityRowFolder_folder else { return nil }
    return SearchFolderHit(
      entityId: row.id,
      icon: iconSpec(row.fragments.entityIcon_entity, kind: .folder),
      path: path(entity.fragments.entityRowPath_entity),
      title: hit.name.map(HighlightedText.init(markup:))
        ?? HighlightedText(plain: EntityText.folderName(folder.name)),
      summary: EntityText.folderSummary(
        folders: folder.folderCount, documents: folder.documentCount))
  }

  private static func iconSpec(_ icon: EntityIcon_entity, kind: EntityKind) -> EntityIconSpec {
    EntityIconSpec(kind: kind, name: icon.icon, color: icon.iconColor)
  }

  private static func path(_ fragment: EntityRowPath_entity) -> [String] {
    fragment.ancestors.compactMap { $0.node.asFolder?.name }.map(EntityText.folderName)
  }
}
