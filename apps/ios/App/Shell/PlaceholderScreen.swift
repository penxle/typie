import Core
import Design
import SwiftUI

struct PlaceholderAction: Identifiable {
  let id = UUID()
  let title: String
  let perform: @MainActor () -> Void
}

struct PlaceholderScreen: View {
  @Environment(\.theme) private var theme
  private var colors: TColors { theme.colors }

  let route: Route
  let actions: [PlaceholderAction]
  var fillerCount = 0

  var body: some View {
    ScrollView {
      VStack(alignment: .leading, spacing: 12) {
        TText(String(describing: route), style: TTypography.caption, color: colors.textHint)
        ForEach(actions) { action in
          TButton(action.title, variant: .secondary) { action.perform() }
        }
        ForEach(0..<fillerCount, id: \.self) { index in
          HStack(spacing: 12) {
            TShapes.rounded(TShapes.md)
              .fill(colors.surfaceInset)
              .frame(width: 44, height: 44)
            VStack(alignment: .leading, spacing: 4) {
              TText("더미 문서 \(index + 1)", style: TTypography.body, color: colors.textDefault)
              TText("스크롤 확인용 자리 표시", style: TTypography.caption, color: colors.textMuted)
            }
          }
          .padding(.vertical, 6)
        }
      }
      .frame(maxWidth: .infinity, alignment: .leading)
      .padding(16)
    }
    .background(colors.surfaceCanvas.ignoresSafeArea())
  }
}
