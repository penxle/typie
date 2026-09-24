#if canImport(UIKit)

  import Core
  import Design
  import FactoryKit
  import SwiftUI

  @MainActor
  struct ProfileScreen: View {
    @Environment(\.theme) private var theme
    private var colors: TColors { theme.colors }

    private let model: ProfileModel
    private let devMode = Container.shared.devMode()
    private let onLogout: () -> Void

    init(model: ProfileModel, onLogout: @escaping () -> Void) {
      self.model = model
      self.onLogout = onLogout
    }

    var body: some View {
      ScrollView {
        VStack(spacing: 16) {
          if let profile = model.profile {
            header(profile)
            logoutRow
          } else if model.loadFailed {
            RetryPrompt { model.refetch() }
          }
          developerSection
        }
        .padding(.horizontal, 16)
        .padding(.vertical, 8)
        .frame(maxWidth: 600)
        .frame(maxWidth: .infinity)
      }
      .canvasBackground()
      .onAppear { model.refetch() }
    }

    private func header(_ profile: ProfileModel.Profile) -> some View {
      HStack(spacing: 16) {
        Img(profile.avatar, side: 72)
          .clipShape(Circle())
          .accessibilityHidden(true)
        VStack(alignment: .leading, spacing: 4) {
          TText(profile.name, style: TTypography.heading, color: colors.textDefault, maxLines: 1)
          TText(profile.email, style: TTypography.control, color: colors.textMuted, maxLines: 1)
        }
        .frame(maxWidth: .infinity, alignment: .leading)
      }
      .padding(20)
      .background(colors.surfaceDefault, in: TShapes.squircle(TShapes.md))
      .accessibilityElement(children: .combine)
    }

    private var logoutRow: some View {
      Button(action: onLogout) {
        TText("로그아웃", style: TTypography.label, color: colors.dangerDefault, maxLines: 1)
          .padding(16)
          .frame(maxWidth: .infinity, alignment: .leading)
          .background(colors.surfaceDefault, in: TShapes.squircle(TShapes.md))
          .contentShape(Rectangle())
      }
      .buttonStyle(TPressEffectStyle(TPressLook()))
    }

    private var developerSection: some View {
      VStack(alignment: .leading, spacing: 0) {
        TText("개발자", style: TTypography.section, color: colors.textMuted)
          .padding(.vertical, 8)
        devModeRow
      }
    }

    private var devModeRow: some View {
      let enabled = devMode.isEnabled
      return Button {
        devMode.setEnabled(!enabled)
      } label: {
        TText("개발자 모드", style: TTypography.label, color: colors.textDefault, maxLines: 1)
          .padding(.leading, CheckCircle.side + 12)
          .frame(maxWidth: .infinity, alignment: .leading)
          .overlay(alignment: .leading) { CheckCircle(on: enabled) }
          .padding(16)
          .background(colors.surfaceDefault, in: TShapes.squircle(TShapes.md))
          .contentShape(Rectangle())
      }
      .buttonStyle(TPressEffectStyle(TPressLook()))
      .accessibilityAddTraits(enabled ? [.isToggle, .isSelected] : .isToggle)
    }
  }

#endif
