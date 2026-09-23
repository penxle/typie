import Core
import FactoryKit
import Foundation
import GraphQL
import Observation

struct FolderContentsInput: Equatable, Sendable {
  let entityId: String
}

struct FolderContentsState: Equatable, Sendable {
  let item: EntityFolderItem
  let characterCount: Int
  let items: [EntityContainerItem]

  init?(_ entity: FolderContents_Query.Data.Entity) {
    guard case .folder(let item)? = EntityRowItem.make(entity.fragments.entityRow_entity, path: [])
    else { return nil }
    self.item = item
    characterCount = entity.node.asFolder?.characterCount ?? 0
    items = entity.children.compactMap { EntityContainerItem.make($0.fragments.entityRow_entity) }
  }
}

@MainActor @Observable
final class FolderContentsStore {
  let entityId: String
  private(set) var folder: FolderContentsState?
  private(set) var hasData = false
  private(set) var loadFailed = false

  @ObservationIgnored var now: () -> Date = { Date() }

  @ObservationIgnored private let initial: EntityFolderItem?
  @ObservationIgnored private let title: String
  @ObservationIgnored private let query: WatchQuery<FolderContentsInput, FolderContents_Query>

  init(entityId: String, initial: EntityFolderItem?, title: String) {
    self.entityId = entityId
    self.initial = initial
    self.title = title
    query = WatchQuery(
      client: Container.shared.graphQLClient(),
      input: { FolderContentsInput(entityId: entityId) },
      query: { FolderContents_Query(entityId: $0.entityId) })
    keepObserving(while: self) { [weak self] in self?.sync() }
  }

  var isPlaceholder: Bool { !hasData && !loadFailed }

  var items: [EntityContainerItem] { folder?.items ?? [] }

  var hero: EntityContainerHeroState {
    let item = folder?.item ?? initial
    let summary: String
    if let folder {
      summary = EntityText.folderMetadataSummary(
        folders: folder.item.folderCount, documents: folder.item.documentCount,
        characters: folder.characterCount)
    } else if let item {
      summary =
        EntityText.folderCounts(folders: item.folderCount, documents: item.documentCount) ?? " "
    } else {
      summary = " "
    }
    return EntityContainerHeroState(
      icon: item?.icon ?? EntityIconSpec(kind: .folder, name: "", color: ""),
      title: item?.title ?? title, summary: summary)
  }

  func refetch() {
    query.refetch()
  }

  private func sync() {
    let data = query.data
    if let data, let next = FolderContentsState(data.entity) {
      if folder != next { folder = next }
      hasData = true
      loadFailed = false
    } else {
      folder = nil
      hasData = false
      if !query.isSettled { loadFailed = false }
    }
    if folder == nil, query.error != nil || query.isSettled {
      loadFailed = true
    }
  }
}
