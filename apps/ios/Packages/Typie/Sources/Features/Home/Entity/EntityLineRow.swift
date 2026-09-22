#if canImport(UIKit)

  import Design
  import SwiftUI

  struct EntityLineRow: View {
    @Environment(\.theme) private var theme

    private let item: EntityRowItem
    private let depth: Int
    private let onOpen: () -> Void

    init(item: EntityRowItem, depth: Int = 0, onOpen: @escaping () -> Void) {
      self.item = item
      self.depth = depth
      self.onOpen = onOpen
    }

    var body: some View {
      let appearance = EntityIcon.appearance(item.icon, colors: theme.colors)
      Button(action: onOpen) {
        HStack(spacing: 12) {
          TIcon(appearance.icon, size: 20, tint: appearance.tint, relativeTo: TTypography.text)
          TText(item.title, style: TTypography.text, color: theme.colors.textDefault, maxLines: 1)
          Spacer(minLength: 0)
        }
        .padding(.leading, HomeTreeView.indent(depth))
        .frame(minHeight: 44)
        .contentShape(Rectangle())
      }
      .buttonStyle(TPressEffectStyle(TPressLook()))
    }
  }

#endif
