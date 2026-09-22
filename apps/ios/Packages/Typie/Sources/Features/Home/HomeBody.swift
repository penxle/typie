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
    private let onOpenRecentAll: () -> Void
    private let onOpenAll: () -> Void
    private let onOpenDocument: (String) -> Void
    private let onOpenFolder: (EntityRowItem) -> Void

    init(
      store: HomeStore, tree: HomeTreeStore, onOpenGoal: @escaping () -> Void,
      onOpenPinnedAll: @escaping () -> Void, onOpenRecentAll: @escaping () -> Void,
      onOpenAll: @escaping () -> Void, onOpenDocument: @escaping (String) -> Void,
      onOpenFolder: @escaping (EntityRowItem) -> Void
    ) {
      self.store = store
      self.tree = tree
      self.onOpenGoal = onOpenGoal
      self.onOpenPinnedAll = onOpenPinnedAll
      self.onOpenRecentAll = onOpenRecentAll
      self.onOpenAll = onOpenAll
      self.onOpenDocument = onOpenDocument
      self.onOpenFolder = onOpenFolder
    }

    var body: some View {
      let site = store.siteOrPlaceholder
      VStack(alignment: .leading, spacing: 0) {
        UserGoalLine(goal: store.goalOrPlaceholder, onOpen: onOpenGoal)
          .padding(.top, 4)
        SectionTitle("고정", onOpen: onOpenPinnedAll)
          .padding(.top, 24)
        if site.pinned.isEmpty {
          TText(
            "문서나 폴더를 고정하면 여기 나타나요", style: TTypography.detail,
            color: theme.colors.textHint, alignment: .center
          )
          .padding(.horizontal, 16)
          .frame(maxWidth: .infinity, minHeight: 80)
          .background(theme.colors.surfaceInset, in: TShapes.squircle(TShapes.md))
        } else {
          ForEach(site.homePinned) { item in
            EntityLineRow(item: item) { open(item) }
          }
        }
        SectionTitle("최근", onOpen: onOpenRecentAll)
          .padding(.top, 24)
        if site.recent.isEmpty {
          RecentEmptyBox()
        } else {
          ForEach(site.recent) { item in
            EntityLineRow(item: item) { open(item) }
          }
        }
        SectionTitle("모두", onOpen: onOpenAll)
          .padding(.top, 24)
        if store.isPlaceholder {
          HomeTreeRows(
            store: tree, nodes: site.roots, depth: 0, onOpenDocument: onOpenDocument,
            onOpenFolder: onOpenFolder)
        } else if site.roots.isEmpty {
          TText("아직 문서가 없어요", style: TTypography.detail, color: theme.colors.textHint)
            .frame(maxWidth: .infinity, minHeight: 44, alignment: .leading)
        } else {
          HomeTreeView(
            store: tree, nodes: site.roots, onOpenDocument: onOpenDocument,
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
