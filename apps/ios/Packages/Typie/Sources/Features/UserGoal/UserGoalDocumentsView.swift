import Core
import Design
import SwiftUI

@MainActor
struct UserGoalDocumentsView: View {
  @Environment(\.theme) private var theme
  private var colors: TColors { theme.colors }

  private let documents: [UserGoalDayDocument]
  private let failed: Bool
  private let onOpen: (UserGoalDayDocument) -> Void

  init(
    documents: [UserGoalDayDocument], failed: Bool, onOpen: @escaping (UserGoalDayDocument) -> Void
  ) {
    self.documents = documents
    self.failed = failed
    self.onOpen = onOpen
  }

  var body: some View {
    VStack(alignment: .leading, spacing: 8) {
      if failed {
        TText("잠시 후 다시 시도해주세요.", style: TTypography.detail, color: colors.textHint)
          .padding(.vertical, 12)
      } else {
        VStack(spacing: 0) {
          ForEach(documents) { document in
            row(document)
          }
        }
      }
    }
  }

  private func row(_ document: UserGoalDayDocument) -> some View {
    Button {
      onOpen(document)
    } label: {
      HStack(spacing: 8) {
        TText(document.title, style: TTypography.control, color: colors.textDefault, maxLines: 1)
          .fontWeight(.regular)
        Spacer(minLength: 8)
        TText(
          "+\(UserGoalFormat.characters(document.additions))", style: TTypography.caption,
          color: colors.textHint, monospacedDigit: true)
        TIcon(
          LucideIcon.chevronRight, size: 14, tint: colors.textHint, relativeTo: TTypography.caption)
      }
      .frame(minHeight: 40)
      .contentShape(Rectangle())
    }
    .buttonStyle(TPressEffectStyle(TPressLook()))
    .accessibilityLabel("\(document.title), \(UserGoalFormat.characters(document.additions))")
  }
}
