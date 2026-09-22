enum UserGoalLineCopy {
  static func text(_ status: UserGoalStatus) -> String {
    var text =
      "오늘 \(Formatting.comma(status.additions)) / \(UserGoalFormat.characters(status.target))"
    if status.streak > 0 { text += " · \(status.streak)일 연속" }
    return text
  }

  static func accessibilityText(_ status: UserGoalStatus?) -> String {
    guard let status else { return "일일 목표, 설정된 목표가 없어요" }
    var text =
      "일일 목표, 오늘 \(UserGoalFormat.characters(status.target)) 중 \(UserGoalFormat.characters(status.additions))"
    if status.streak > 0 { text += ", \(status.streak)일 연속" }
    return text
  }
}

#if canImport(UIKit)

  import Core
  import Design
  import SwiftUI

  struct UserGoalLine: View {
    @Environment(\.theme) private var theme
    private var colors: TColors { theme.colors }

    private let goal: UserGoalState
    private let onOpen: () -> Void

    init(goal: UserGoalState, onOpen: @escaping () -> Void) {
      self.goal = goal
      self.onOpen = onOpen
    }

    var body: some View {
      Button(action: onOpen) {
        HStack(spacing: 8) {
          if let status = goal.status {
            TProgressRing(
              progress: Double(status.additions) / Double(status.target),
              state: status.achieved ? .achieved : .under, size: 20, relativeTo: TTypography.text)
            TText(
              UserGoalLineCopy.text(status), style: TTypography.text, color: colors.textMuted,
              maxLines: 1,
              monospacedDigit: true)
          } else {
            TProgressRing(progress: 0, state: .noGoal, size: 20, relativeTo: TTypography.text)
            TText(
              "설정된 목표가 없어요", style: TTypography.text, color: colors.textMuted, maxLines: 1)
          }
          TIcon(
            LucideIcon.chevronRight, size: 18, tint: colors.textHint, relativeTo: TTypography.text
          )
          Spacer(minLength: 0)
        }
        .frame(minHeight: 44)
        .contentShape(Rectangle())
      }
      .buttonStyle(TPressEffectStyle(TPressLook()))
      .accessibilityElement(children: .combine)
      .accessibilityLabel(UserGoalLineCopy.accessibilityText(goal.status))
    }
  }

#endif
