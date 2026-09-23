#if canImport(UIKit)

  import SwiftUI

  @MainActor
  struct FolderScreen: View {
    private let store: FolderContentsStore
    private let title: HeroTitleState
    private let onOpenDocument: (String) -> Void
    private let onOpenFolder: (EntityFolderItem) -> Void

    init(
      store: FolderContentsStore, title: HeroTitleState,
      onOpenDocument: @escaping (String) -> Void,
      onOpenFolder: @escaping (EntityFolderItem) -> Void
    ) {
      self.store = store
      self.title = title
      self.onOpenDocument = onOpenDocument
      self.onOpenFolder = onOpenFolder
    }

    var body: some View {
      EntityContainerScreen(
        title: title, hero: store.hero, items: store.items, now: store.now(),
        isPlaceholder: store.isPlaceholder, showsRetry: store.loadFailed && !store.hasData,
        emptyText: "폴더가 비어 있어요", bottomClearance: HomeBody.bottomClearance,
        onRetry: { store.refetch() },
        onOpenDocument: onOpenDocument, onOpenFolder: onOpenFolder)
    }
  }

#endif
