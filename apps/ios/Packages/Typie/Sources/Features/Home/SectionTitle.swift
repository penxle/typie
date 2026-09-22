#if canImport(UIKit)

  import Design
  import SwiftUI

  struct SectionTitle: View {
    @Environment(\.theme) private var theme

    private let title: String
    private let onOpen: () -> Void

    init(_ title: String, onOpen: @escaping () -> Void) {
      self.title = title
      self.onOpen = onOpen
    }

    var body: some View {
      Button(action: onOpen) {
        HStack(spacing: 2) {
          TText(title, style: TTypography.heading, color: theme.colors.textDefault)
          TIcon(
            LucideIcon.chevronRight, size: 20, tint: theme.colors.textHint,
            relativeTo: TTypography.heading)
        }
        .frame(minHeight: 44)
        .contentShape(Rectangle())
      }
      .buttonStyle(TPressEffectStyle(TPressLook()))
      .accessibilityLabel("\(title) 전체 보기")
    }
  }

#endif
