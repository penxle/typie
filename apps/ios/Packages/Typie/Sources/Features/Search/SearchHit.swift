import Core
import Foundation
import GraphQL

struct SearchDocumentHit: Equatable, Sendable {
  let entityId: String
  let icon: EntityIconSpec
  let path: [String]
  let title: HighlightedText
  let subtitle: HighlightedText?
  let preview: HighlightedText
  let previewLines: Int
  let updatedAt: Date?
}

struct SearchFolderHit: Equatable, Sendable {
  let entityId: String
  let icon: EntityIconSpec
  let path: [String]
  let title: HighlightedText
  let summary: String
}

enum SearchHit: Equatable, Sendable, Identifiable {
  case document(SearchDocumentHit)
  case folder(SearchFolderHit)

  var entityId: String {
    switch self {
    case .document(let hit): hit.entityId
    case .folder(let hit): hit.entityId
    }
  }

  var id: String { entityId }
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
      icon: EntityRowItem.iconSpec(row.fragments.entityIcon_entity, kind: .document),
      path: EntityRowItem.path(entity.fragments.entityRowPath_entity),
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
      icon: EntityRowItem.iconSpec(row.fragments.entityIcon_entity, kind: .folder),
      path: EntityRowItem.path(entity.fragments.entityRowPath_entity),
      title: hit.name.map(HighlightedText.init(markup:))
        ?? HighlightedText(plain: EntityText.folderName(folder.name)),
      summary: EntityText.folderSummary(
        folders: folder.folderCount, documents: folder.documentCount))
  }
}
