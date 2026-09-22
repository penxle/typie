import Core
import Design
import SwiftUI

@MainActor
struct UserGoalDayView: View {
  static let ringSize: CGFloat = 160

  @Environment(\.theme) private var theme
  @Environment(\.accessibilityReduceMotion) private var reduceMotion
  private var colors: TColors { theme.colors }

  private let day: UserGoalDay

  init(day: UserGoalDay) {
    self.day = day
  }

  var body: some View {
    VStack(spacing: 20) {
      TText(day.title, style: TTypography.heading, color: colors.textDefault, monospacedDigit: true)
      if day.hasGoal {
        ring
        VStack(spacing: 2) {
          TText(
            day.countText, style: TTypography.title,
            color: day.achieved ? colors.successDefault : colors.textDefault, monospacedDigit: true)
          TText(
            day.sentence, style: TTypography.detail,
            color: day.achieved ? colors.textDefault : colors.textMuted, monospacedDigit: true)
        }
      } else {
        TProgressRing(progress: 0, state: .noGoal, size: Self.ringSize)
        VStack(spacing: 2) {
          TText(
            day.countText, style: TTypography.title, color: colors.textMuted, monospacedDigit: true)
          TText(day.sentence, style: TTypography.detail, color: colors.textMuted)
        }
      }
    }
    .frame(maxWidth: .infinity)
    .accessibilityElement(children: .ignore)
    .accessibilityLabel(day.accessibilityLabel)
    .id(day.day)
    .transition(reduceMotion ? .identity : .opacity)
  }

  private var ring: some View {
    ZStack {
      TProgressRing(
        progress: day.progress, state: day.achieved ? .achieved : .under, size: Self.ringSize)
      if day.achieved {
        TIcon(LucideIcon.check, size: 48, tint: colors.successDefault)
          .transition(reduceMotion ? .identity : .scale(scale: 0.6).combined(with: .opacity))
      } else {
        TText(
          day.percentText, style: TTypography.hero, color: colors.textDefault, monospacedDigit: true
        )
      }
    }
    .frame(width: Self.ringSize, height: Self.ringSize)
  }
}
