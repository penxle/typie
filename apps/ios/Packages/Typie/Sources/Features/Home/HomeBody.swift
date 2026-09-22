#if canImport(UIKit)

  import Design
  import SwiftUI

  @MainActor
  struct HomeBody: View {
    @Environment(\.theme) private var theme

    static let bottomClearance: CGFloat = 62

    private let store: HomeStore
    private let tree: HomeTreeStore
    private let onOpenGoal: () -> Void
    private let onOpenPinnedAll: () -> Void
    private let onOpenAll: () -> Void
    private let onOpenDocument: (String) -> Void
    private let onOpenFolder: (EntityRowItem) -> Void

    init(
      store: HomeStore, tree: HomeTreeStore, onOpenGoal: @escaping () -> Void,
      onOpenPinnedAll: @escaping () -> Void, onOpenAll: @escaping () -> Void,
      onOpenDocument: @escaping (String) -> Void, onOpenFolder: @escaping (EntityRowItem) -> Void
    ) {
      self.store = store
      self.tree = tree
      self.onOpenGoal = onOpenGoal
      self.onOpenPinnedAll = onOpenPinnedAll
      self.onOpenAll = onOpenAll
      self.onOpenDocument = onOpenDocument
      self.onOpenFolder = onOpenFolder
    }

    var body: some View {
      let site = store.siteOrPlaceholder
      VStack(alignment: .leading, spacing: 0) {
        UserGoalLine(goal: store.goalOrPlaceholder, onOpen: onOpenGoal)
          .padding(.top, 4)
        if !site.pinned.isEmpty {
          SectionTitle("고정", onOpen: onOpenPinnedAll)
            .padding(.top, 24)
          ForEach(site.homePinned) { item in
            EntityLineRow(item: item) { open(item) }
          }
        }
        SectionTitle("모두", onOpen: onOpenAll)
          .padding(.top, 24)
        if site.roots.isEmpty {
          TText("아직 문서가 없어요", style: TTypography.detail, color: theme.colors.textHint)
            .frame(maxWidth: .infinity, minHeight: 44, alignment: .leading)
        } else {
          HomeTreeView(
            store: tree, nodes: site.roots, depth: 0, onOpenDocument: onOpenDocument,
            onOpenFolder: onOpenFolder)
        }
      }
    }

    private func open(_ item: EntityRowItem) {
      switch item {
      case .document: onOpenDocument(item.entityId)
      case .folder: onOpenFolder(item)
      }
    }
  }

#endif
