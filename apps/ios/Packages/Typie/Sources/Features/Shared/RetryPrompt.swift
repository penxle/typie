import Design
import SwiftUI

struct RetryPrompt: View {
  @Environment(\.theme) private var theme
  private var colors: TColors { theme.colors }

  private let retry: () -> Void

  init(retry: @escaping () -> Void) {
    self.retry = retry
  }

  var body: some View {
    VStack(spacing: 0) {
      TText("문제가 발생했어요", style: TTypography.label, color: colors.textDefault)
      Spacer().frame(height: 6)
      TText("잠시 후 다시 시도해주세요.", style: TTypography.caption, color: colors.textMuted)
      Spacer().frame(height: 20)
      TButton("다시 시도", variant: .secondary) { retry() }
    }
    .frame(maxWidth: .infinity)
    .padding(.vertical, 24)
  }
}
