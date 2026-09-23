#if canImport(UIKit)

  import Design
  import FactoryKit
  import SwiftUI

  @MainActor
  struct SiteSwitcherScreen: View {
    @Environment(\.theme) private var theme
    private var colors: TColors { theme.colors }

    private let model = Container.shared.sites()
    private let onSelected: () -> Void
    private let onCreate: () -> Void

    init(onSelected: @escaping () -> Void, onCreate: @escaping () -> Void) {
      self.onSelected = onSelected
      self.onCreate = onCreate
    }

    var body: some View {
      ScrollView {
        VStack(spacing: 12) {
          if model.loadFailed {
            RetryPrompt { model.refetch() }
          } else {
            if !model.sites.isEmpty {
              card {
                ForEach(model.sites) { site in
                  Button {
                    model.select(site.id)
                    onSelected()
                  } label: {
                    row(site, isCurrent: site.id == model.current?.id)
                  }
                  .buttonStyle(TPressEffectStyle(TPressLook()))
                }
              }
            }
            if model.isSettled {
              card {
                Button(action: onCreate) { createRow }
                  .buttonStyle(TPressEffectStyle(TPressLook()))
              }
            }
          }
        }
        .padding(.horizontal, 16)
        .padding(.vertical, 8)
        .frame(maxWidth: 600)
        .frame(maxWidth: .infinity)
      }
      .scrollBounceBehavior(.basedOnSize)
      .sheetBackground()
      .onAppear { model.refetch() }
    }

    private func row(_ site: Site, isCurrent: Bool) -> some View {
      HStack(spacing: 12) {
        Img(site.logo, side: 32)
          .clipShape(TShapes.rounded(TShapes.sm))
          .accessibilityHidden(true)
        VStack(alignment: .leading, spacing: 0) {
          TText(site.name, style: TTypography.label, color: colors.textDefault, maxLines: 1)
          TText(site.url, style: TTypography.caption, color: colors.textMuted, maxLines: 1)
        }
        Spacer(minLength: 12)
        if isCurrent {
          TIcon(
            LucideIcon.check, size: 18, tint: colors.textDefault, relativeTo: TTypography.label)
        }
      }
      .padding(.vertical, 10)
      .contentShape(Rectangle())
      .accessibilityElement(children: .combine)
      .accessibilityAddTraits(isCurrent ? [.isSelected] : [])
    }

    private func card(@ViewBuilder _ content: () -> some View) -> some View {
      VStack(spacing: 0) { content() }
        .padding(.horizontal, 10)
        .background(
          colors.textDefault.opacity(Self.cardInsetOpacity), in: TShapes.squircle(TShapes.lg))
    }

    static let cardInsetOpacity: Double = 0.03

    private var createRow: some View {
      HStack(spacing: 12) {
        TIcon(LucideIcon.plus, size: 18, tint: colors.textMuted)
          .frame(width: 32, height: 32)
        TText("새 스페이스 생성", style: TTypography.label, color: colors.textMuted)
        Spacer(minLength: 0)
      }
      .padding(.vertical, 10)
      .contentShape(Rectangle())
      .accessibilityElement(children: .combine)
    }
  }

#endif
