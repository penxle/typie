#if canImport(UIKit)

  import SwiftUI

  @MainActor
  struct SiteEntitiesScreen: View {
    private let store: SiteEntitiesStore
    private let title: HeroTitleState
    private let initialName: String?
    private let onOpenDocument: (String) -> Void
    private let onOpenFolder: (EntityFolderItem) -> Void

    init(
      store: SiteEntitiesStore, title: HeroTitleState, initialName: String?,
      onOpenDocument: @escaping (String) -> Void,
      onOpenFolder: @escaping (EntityFolderItem) -> Void
    ) {
      self.store = store
      self.title = title
      self.initialName = initialName
      self.onOpenDocument = onOpenDocument
      self.onOpenFolder = onOpenFolder
    }

    var body: some View {
      let site = store.site
      EntityContainerScreen(
        title: title,
        hero: EntityContainerHeroState(
          icon: nil, title: site?.name ?? initialName ?? " ", summary: site?.summary ?? " "),
        items: site?.items ?? [], now: store.now(), isPlaceholder: store.isPlaceholder,
        showsRetry: store.loadFailed && !store.hasData,
        emptyText: "아직 문서를 만들지 않았어요.\n오른쪽 아래 만들기 버튼으로 새 문서를 만들어보세요.",
        bottomClearance: HomeBody.bottomClearance, onRetry: { store.refetch() },
        onOpenDocument: onOpenDocument, onOpenFolder: onOpenFolder)
    }
  }

#endif
