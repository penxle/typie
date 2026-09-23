#if canImport(UIKit)

  import Design
  import SwiftUI

  @MainActor
  struct HomeBody: View {
    @Environment(\.theme) private var theme
    @Environment(\.accessibilityReduceMotion) private var reduceMotion

    static let bottomClearance: CGFloat = 62

    private let store: HomeStore
    private let tree: HomeTreeStore
    private let layout: HomeLayoutStore
    private let onOpenGoal: () -> Void
    private let onOpenPinnedAll: () -> Void
    private let onOpenRecentAll: () -> Void
    private let onOpenAll: () -> Void
    private let onOpenDocument: (String) -> Void
    private let onOpenFolder: (EntityRowItem) -> Void

    init(
      store: HomeStore, tree: HomeTreeStore, layout: HomeLayoutStore,
      onOpenGoal: @escaping () -> Void,
      onOpenPinnedAll: @escaping () -> Void, onOpenRecentAll: @escaping () -> Void,
      onOpenAll: @escaping () -> Void, onOpenDocument: @escaping (String) -> Void,
      onOpenFolder: @escaping (EntityRowItem) -> Void
    ) {
      self.store = store
      self.tree = tree
      self.layout = layout
      self.onOpenGoal = onOpenGoal
      self.onOpenPinnedAll = onOpenPinnedAll
      self.onOpenRecentAll = onOpenRecentAll
      self.onOpenAll = onOpenAll
      self.onOpenDocument = onOpenDocument
      self.onOpenFolder = onOpenFolder
    }

    var body: some View {
      let site = store.siteOrPlaceholder
      if store.isPlaceholder {
        sections(site)
      } else {
        sections(site)
          .animation(
            HomeTreeView.expandAnimation(reduceMotion: reduceMotion), value: tree.expansion)
      }
    }

    private func sections(_ site: HomeSiteState) -> some View {
      let visible = layout.visible
      return VStack(alignment: .leading, spacing: 0) {
        ForEach(visible) { section in
          switch section {
          case .goal:
            if visible.first == .goal {
              UserGoalLine(goal: store.goalOrPlaceholder, onOpen: onOpenGoal)
                .padding(.top, 4)
            } else {
              SectionTitle("목표", onOpen: onOpenGoal)
                .padding(.top, 24)
              UserGoalLine(goal: store.goalOrPlaceholder, showsChevron: false, onOpen: onOpenGoal)
            }
          case .pinned:
            SectionTitle("고정", onOpen: onOpenPinnedAll)
              .padding(.top, 24)
            if site.pinned.isEmpty {
              EmptyStateBox(text: "문서나 폴더를 고정해 빠르게 접근할 수 있어요")
            } else {
              ForEach(site.homePinned) { item in
                EntityLineRow(item: item) { open(item) }
              }
            }
          case .recent:
            SectionTitle("최근", onOpen: onOpenRecentAll)
              .padding(.top, 24)
            let recent = site.recent(layout.recentSort)
            if recent.isEmpty {
              EmptyStateBox(text: layout.recentSort.emptyText)
            } else {
              ForEach(recent) { item in
                EntityLineRow(item: item) { open(item) }
              }
            }
          case .all:
            SectionTitle("전체", onOpen: onOpenAll)
              .padding(.top, 24)
            if store.isPlaceholder {
              HomeTreeRows(
                store: tree, nodes: site.roots, depth: 0, onOpenDocument: onOpenDocument,
                onOpenFolder: onOpenFolder)
            } else if site.roots.isEmpty {
              EmptyStateBox(text: "아직 문서를 만들지 않았어요.\n오른쪽 아래 만들기 버튼으로 새 문서를 만들어보세요.")
            } else {
              HomeTreeView(
                store: tree, nodes: site.roots, onOpenDocument: onOpenDocument,
                onOpenFolder: onOpenFolder)
            }
          }
        }
      }
      .animation(HomeTreeView.expandAnimation(reduceMotion: reduceMotion), value: visible)
    }

    private func open(_ item: EntityRowItem) {
      switch item {
      case .document: onOpenDocument(item.entityId)
      case .folder: onOpenFolder(item)
      }
    }
  }

#endif
