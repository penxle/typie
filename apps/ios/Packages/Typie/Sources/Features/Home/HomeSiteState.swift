import Foundation
import GraphQL

struct HomeSiteState: Equatable, Sendable {
  static let pinnedLimit = 5

  let name: String
  let pinned: [EntityRowItem]
  let recent: [EntityRowItem]
  let roots: [HomeTreeNode]

  init(name: String, pinned: [EntityRowItem], recent: [EntityRowItem], roots: [HomeTreeNode]) {
    self.name = name
    self.pinned = pinned
    self.recent = recent
    self.roots = roots
  }

  init(_ site: HomeScreen_site) {
    name = site.name
    pinned = site.pinnedEntities.compactMap {
      EntityRowItem.make($0.fragments.homeTree_entity.fragments.entityRow_entity, path: [])
    }
    recent = site.recentDocuments.documents.compactMap {
      RecentDocument.make($0.fragments.recentDocument_document)?.item
    }
    roots = site.entities.compactMap { HomeTreeNode.make($0.fragments.homeTree_entity) }
  }

  var homePinned: [EntityRowItem] { Array(pinned.prefix(Self.pinnedLimit)) }

  static let placeholder: HomeSiteState = {
    let rows = (1...3).map { index in
      EntityRowItem.document(
        EntityDocumentItem(
          entityId: "placeholder-\(index)",
          icon: EntityIconSpec(kind: .document, name: "", color: ""), path: [],
          title: EntityText.documentTitle(""), updatedAt: nil))
    }
    return HomeSiteState(
      name: "", pinned: rows, recent: rows, roots: rows.map(HomeTreeNode.document))
  }()
}
