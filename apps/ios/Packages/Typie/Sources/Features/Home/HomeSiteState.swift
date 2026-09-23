import Foundation
import GraphQL

struct HomeSiteState: Equatable, Sendable {
  static let pinnedLimit = 5

  let name: String
  let pinned: [EntityRowItem]
  let recentlyViewed: [EntityRowItem]
  let recentlyUpdated: [EntityRowItem]
  let roots: [HomeTreeNode]

  init(
    name: String, pinned: [EntityRowItem], recentlyViewed: [EntityRowItem],
    recentlyUpdated: [EntityRowItem], roots: [HomeTreeNode]
  ) {
    self.name = name
    self.pinned = pinned
    self.recentlyViewed = recentlyViewed
    self.recentlyUpdated = recentlyUpdated
    self.roots = roots
  }

  func recent(_ sort: RecentSort) -> [EntityRowItem] {
    switch sort {
    case .viewed: recentlyViewed
    case .updated: recentlyUpdated
    }
  }

  init(_ site: HomeScreen_site) {
    name = site.name
    pinned = site.pinnedEntities.compactMap {
      EntityRowItem.make($0.fragments.homeTree_entity.fragments.entityRow_entity, path: [])
    }
    recentlyViewed = site.recentlyViewedDocuments.documents.compactMap {
      RecentDocument.make($0.fragments.recentDocument_document)?.item
    }
    recentlyUpdated = site.recentlyUpdatedDocuments.documents.compactMap {
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
          title: EntityText.documentTitle(""), subtitle: nil, excerpt: "", updatedAt: nil))
    }
    return HomeSiteState(
      name: "", pinned: rows, recentlyViewed: rows, recentlyUpdated: rows,
      roots: rows.map(HomeTreeNode.document))
  }()
}
