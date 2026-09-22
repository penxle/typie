import Core
import Design
import SwiftUI

struct PlaceholderScreen: View {
  @Environment(\.theme) private var theme
  private var colors: TColors { theme.colors }

  let route: Route

  var body: some View {
    ScrollView {
      LazyVStack(alignment: .leading, spacing: 12) {
        TText(String(describing: route), style: TTypography.caption, color: colors.textHint)
      }
      .frame(maxWidth: .infinity, alignment: .leading)
      .padding(16)
    }
    .canvasBackground()
  }
}
