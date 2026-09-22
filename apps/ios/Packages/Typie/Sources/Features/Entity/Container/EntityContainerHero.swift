#if canImport(UIKit)

  import Design
  import SwiftUI

  struct EntityContainerHero: View {
    @Environment(\.theme) private var theme

    let icon: EntityIconSpec?
    let title: String

    var body: some View {
      HStack(spacing: 12) {
        if let icon {
          let appearance = EntityIcon.appearance(icon, colors: theme.colors)
          TIcon(appearance.icon, size: 28, tint: appearance.tint, relativeTo: TTypography.hero)
            .accessibilityHidden(true)
        }
        TText(title, style: TTypography.hero, color: theme.colors.textDefault, maxLines: 1)
      }
      .padding(.horizontal, 16)
      .padding(.top, 12)
      .padding(.bottom, 24)
      .frame(maxWidth: .infinity, alignment: .leading)
      .accessibilityElement(children: .combine)
    }
  }

#endif
