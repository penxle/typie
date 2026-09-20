#if canImport(UIKit)

  import Design
  import SwiftUI

  @MainActor
  struct FolderScreen: View {
    @Environment(\.theme) private var theme

    private let folderId: String
    private let tree: HomeTreeStore
    private let onOpenDocument: (String) -> Void
    private let onOpenFolder: (EntityRowItem) -> Void

    init(
      folderId: String, tree: HomeTreeStore, onOpenDocument: @escaping (String) -> Void,
      onOpenFolder: @escaping (EntityRowItem) -> Void
    ) {
      self.folderId = folderId
      self.tree = tree
      self.onOpenDocument = onOpenDocument
      self.onOpenFolder = onOpenFolder
    }

    var body: some View {
      let children = tree.children(of: folderId)
      ScrollView {
        VStack(alignment: .leading, spacing: 0) {
          if tree.failed(folderId) {
            RetryPrompt { tree.refetchChildren(of: folderId) }
          } else if children.isEmpty, tree.isLoaded(folderId) {
            TText("아직 문서가 없어요", style: TTypography.detail, color: theme.colors.textHint)
              .frame(maxWidth: .infinity, minHeight: 44, alignment: .leading)
          } else {
            HomeTreeView(
              store: tree, nodes: children, depth: 0, onOpenDocument: onOpenDocument,
              onOpenFolder: onOpenFolder)
          }
        }
        .padding(.horizontal, 16)
        .padding(.vertical, 8)
        .frame(maxWidth: 600)
        .frame(maxWidth: .infinity)
      }
      .canvasBackground()
      .onAppear { tree.ensureLoaded(folderId) }
    }
  }

#endif
