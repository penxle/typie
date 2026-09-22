#if canImport(UIKit)

  import Design
  import SwiftUI

  struct EmptyStateBox: View {
    @Environment(\.theme) private var theme

    let text: String

    var body: some View {
      TText(text, style: TTypography.detail, color: theme.colors.textHint, alignment: .center)
        .padding(.horizontal, 16)
        .frame(maxWidth: .infinity, minHeight: 120)
        .background(theme.colors.surfaceInset, in: TShapes.squircle(TShapes.md))
    }
  }

#endif
