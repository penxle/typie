#if canImport(UIKit)

  import Design
  import SwiftUI

  @MainActor
  struct HomeTreeView: View {
    @Environment(\.accessibilityReduceMotion) private var reduceMotion

    static let indentStep: CGFloat = 24
    static let expand = Animation.spring(duration: 0.22, bounce: 0)
    static let expandReduced = Animation.easeOut(duration: 0.12)

    static func expandAnimation(reduceMotion: Bool) -> Animation {
      reduceMotion ? expandReduced : expand
    }

    static func rowTransition(reduceMotion: Bool) -> AnyTransition {
      if reduceMotion { return .opacity }
      return .asymmetric(
        insertion: .opacity.animation(.easeOut(duration: 0.12).delay(0.05)),
        removal: .opacity.animation(.easeOut(duration: 0.08)))
    }

    static func indent(_ depth: Int) -> CGFloat { CGFloat(depth) * indentStep }

    private let store: HomeTreeStore
    private let nodes: [HomeTreeNode]
    private let onOpenDocument: (String) -> Void
    private let onOpenFolder: (EntityRowItem) -> Void

    init(
      store: HomeTreeStore, nodes: [HomeTreeNode],
      onOpenDocument: @escaping (String) -> Void, onOpenFolder: @escaping (EntityRowItem) -> Void
    ) {
      self.store = store
      self.nodes = nodes
      self.onOpenDocument = onOpenDocument
      self.onOpenFolder = onOpenFolder
    }

    var body: some View {
      VStack(alignment: .leading, spacing: 0) {
        HomeTreeRows(
          store: store, nodes: nodes, depth: 0, onOpenDocument: onOpenDocument,
          onOpenFolder: onOpenFolder)
      }
      .animation(Self.expandAnimation(reduceMotion: reduceMotion), value: store.expansion)
    }
  }

  @MainActor
  struct HomeTreeRows: View {
    @Environment(\.accessibilityReduceMotion) private var reduceMotion

    private let store: HomeTreeStore
    private let nodes: [HomeTreeNode]
    private let hint: String?
    private let depth: Int
    private let onOpenDocument: (String) -> Void
    private let onOpenFolder: (EntityRowItem) -> Void

    init(
      store: HomeTreeStore, nodes: [HomeTreeNode], hint: String? = nil, depth: Int,
      onOpenDocument: @escaping (String) -> Void, onOpenFolder: @escaping (EntityRowItem) -> Void
    ) {
      self.store = store
      self.nodes = nodes
      self.hint = hint
      self.depth = depth
      self.onOpenDocument = onOpenDocument
      self.onOpenFolder = onOpenFolder
    }

    var body: some View {
      let transition =
        depth > 0 ? HomeTreeView.rowTransition(reduceMotion: reduceMotion) : AnyTransition.identity
      ForEach(nodes) { node in
        row(node).transition(transition)
      }
      if let hint {
        TreeHintRow(text: hint, depth: depth).transition(transition)
      }
    }

    @ViewBuilder
    private func row(_ node: HomeTreeNode) -> some View {
      switch node {
      case .document(let item):
        EntityLineRow(item: item, depth: depth) { onOpenDocument(item.entityId) }
      case .divider:
        TreeDividerRow(depth: depth)
      case .folder(let item, let childCount):
        let expanded = store.isExpanded(item.entityId)
        FolderRow(
          item: item, childCount: childCount, depth: depth, expanded: expanded,
          onOpen: { onOpenFolder(item) }, onToggle: { store.toggle(item.entityId) })
        if expanded {
          HomeTreeRows(
            store: store, nodes: store.children(of: item.entityId),
            hint: childrenHint(item.entityId), depth: depth + 1,
            onOpenDocument: onOpenDocument, onOpenFolder: onOpenFolder)
        }
      }
    }
  }

  extension HomeTreeRows {
    fileprivate func childrenHint(_ id: String) -> String? {
      if store.failed(id) { return "폴더 내용을 불러오지 못했어요" }
      if store.isLoaded(id), store.children(of: id).isEmpty { return "폴더가 비어있어요" }
      return nil
    }
  }

  private struct TreeHintRow: View {
    @Environment(\.theme) private var theme

    let text: String
    let depth: Int

    var body: some View {
      TText(text, style: TTypography.detail, color: theme.colors.textHint)
        .padding(.leading, HomeTreeView.indent(depth))
        .frame(maxWidth: .infinity, minHeight: 44, alignment: .leading)
    }
  }

  private struct FolderRow: View {
    @Environment(\.theme) private var theme
    @Environment(\.accessibilityReduceMotion) private var reduceMotion
    private var colors: TColors { theme.colors }

    let item: EntityRowItem
    let childCount: Int
    let depth: Int
    let expanded: Bool
    let onOpen: () -> Void
    let onToggle: () -> Void

    var body: some View {
      let appearance = EntityIcon.appearance(item.icon, colors: colors)
      HStack(spacing: 0) {
        Button(action: onOpen) {
          HStack(spacing: 12) {
            TIcon(appearance.icon, size: 20, tint: appearance.tint, relativeTo: TTypography.text)
            TText(item.title, style: TTypography.text, color: colors.textDefault, maxLines: 1)
            Spacer(minLength: 0)
          }
          .padding(.leading, HomeTreeView.indent(depth))
          .frame(minHeight: 44)
          .contentShape(Rectangle())
        }
        .buttonStyle(TPressEffectStyle(TPressLook()))
        Button(action: onToggle) {
          HStack(spacing: 6) {
            TText(
              "\(childCount)", style: TTypography.detail, color: colors.textHint,
              monospacedDigit: true)
            TIcon(
              LucideIcon.chevronDown, size: 18, tint: colors.textHint, relativeTo: TTypography.text
            )
            .rotationEffect(.degrees(expanded ? 0 : -90))
            .animation(reduceMotion ? nil : HomeTreeView.expand, value: expanded)
          }
          .padding(.leading, 12)
          .frame(minWidth: 44, minHeight: 44)
          .contentShape(Rectangle())
        }
        .buttonStyle(TPressEffectStyle(TPressLook()))
        .accessibilityLabel("항목 \(childCount)개, \(expanded ? "접기" : "펼치기")")
        .accessibilityValue(expanded ? "펼침" : "접힘")
      }
    }
  }

  private struct TreeDividerRow: View {
    @Environment(\.theme) private var theme

    let depth: Int

    var body: some View {
      Rectangle()
        .fill(theme.colors.borderDefault)
        .frame(height: 1)
        .frame(maxWidth: .infinity, minHeight: 44)
        .padding(.leading, HomeTreeView.indent(depth))
        .contentShape(Rectangle())
        .onTapGesture {}
        .accessibilityHidden(true)
    }
  }

#endif
