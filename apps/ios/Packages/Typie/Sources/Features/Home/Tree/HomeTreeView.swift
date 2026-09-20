#if canImport(UIKit)

  import Design
  import SwiftUI

  @MainActor
  struct HomeTreeView: View {
    @Environment(\.accessibilityReduceMotion) private var reduceMotion

    static let indentStep: CGFloat = 24
    static let expandCurve = Animation.timingCurve(0.2, 0.8, 0.2, 1, duration: 0.22)

    static func indent(_ depth: Int) -> CGFloat { CGFloat(depth) * indentStep }

    private let store: HomeTreeStore
    private let nodes: [HomeTreeNode]
    private let depth: Int
    private let onOpenDocument: (String) -> Void
    private let onOpenFolder: (EntityRowItem) -> Void

    init(
      store: HomeTreeStore, nodes: [HomeTreeNode], depth: Int,
      onOpenDocument: @escaping (String) -> Void, onOpenFolder: @escaping (EntityRowItem) -> Void
    ) {
      self.store = store
      self.nodes = nodes
      self.depth = depth
      self.onOpenDocument = onOpenDocument
      self.onOpenFolder = onOpenFolder
    }

    var body: some View {
      ForEach(nodes) { node in
        row(node)
          .transition(.move(edge: .top).combined(with: .opacity))
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
          onOpen: { onOpenFolder(item) }, onToggle: { toggle(item.entityId) })
        if expanded {
          let children = store.children(of: item.entityId)
          HomeTreeView(
            store: store, nodes: children, depth: depth + 1, onOpenDocument: onOpenDocument,
            onOpenFolder: onOpenFolder
          )
          .animation(reduceMotion ? nil : Self.expandCurve, value: children.map(\.id))
        }
      }
    }

    private func toggle(_ id: String) {
      if !store.isExpanded(id), !reduceMotion {
        withAnimation(Self.expandCurve) { store.toggle(id) }
      } else {
        store.toggle(id)
      }
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
            .animation(
              reduceMotion ? nil : .timingCurve(0.2, 0.8, 0.2, 1, duration: 0.2), value: expanded)
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
        .fill(theme.colors.borderHairline)
        .frame(height: 1)
        .frame(height: 16)
        .padding(.leading, HomeTreeView.indent(depth))
        .accessibilityHidden(true)
    }
  }

#endif
