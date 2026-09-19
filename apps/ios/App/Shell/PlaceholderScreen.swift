import Core
import Design
import SwiftUI

struct PlaceholderAction: Identifiable {
  let id = UUID()
  let title: String
  let perform: @MainActor () -> Void
}

struct PlaceholderDevActions: View {
  let actions: [PlaceholderAction]

  var body: some View {
    VStack(alignment: .leading, spacing: 12) {
      ForEach(actions) { action in
        TButton(action.title, variant: .secondary) { action.perform() }
      }
    }
    .frame(maxWidth: .infinity, alignment: .leading)
  }
}

struct PlaceholderScreen: View {
  @Environment(\.theme) private var theme
  private var colors: TColors { theme.colors }

  let route: Route
  let actions: [PlaceholderAction]

  var body: some View {
    ScrollView {
      LazyVStack(alignment: .leading, spacing: 12) {
        TText(String(describing: route), style: TTypography.caption, color: colors.textHint)
        PlaceholderDevActions(actions: actions)
      }
      .frame(maxWidth: .infinity, alignment: .leading)
      .padding(16)
    }
    .canvasBackground()
  }
}
