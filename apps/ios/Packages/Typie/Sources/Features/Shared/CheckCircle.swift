import Design
import SwiftUI

struct CheckCircle: View {
  static let side: CGFloat = 22

  @Environment(\.theme) private var theme
  @Environment(\.accessibilityReduceMotion) private var reduceMotion
  private var colors: TColors { theme.colors }

  let on: Bool

  var body: some View {
    ZStack {
      Circle()
        .strokeBorder(colors.borderEmphasis, lineWidth: 1.5)
        .opacity(on ? 0 : 1)
      Circle()
        .fill(colors.accentDefault)
        .scaleEffect(on || reduceMotion ? 1 : 0.6)
        .opacity(on ? 1 : 0)
      TIcon(LucideIcon.check, size: 12, tint: colors.surfaceCanvas)
        .opacity(on ? 1 : 0)
    }
    .frame(width: Self.side, height: Self.side)
    .contentShape(Rectangle())
    .animation(.easeOut(duration: 0.12), value: on)
  }
}
