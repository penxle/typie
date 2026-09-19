#if canImport(UIKit)

  import Design
  import FactoryKit
  import SwiftUI

  @MainActor
  public struct SiteSwitcherScreen: View {
    @Environment(\.theme) private var theme
    private var colors: TColors { theme.colors }

    private let model = Container.shared.sites()
    private let onSelected: () -> Void
    private let onCreate: () -> Void

    public init(onSelected: @escaping () -> Void, onCreate: @escaping () -> Void) {
      self.onSelected = onSelected
      self.onCreate = onCreate
    }

    public var body: some View {
      ScrollView {
        VStack(spacing: 12) {
          if model.loadFailed {
            VStack(spacing: 0) {
              TText("문제가 발생했어요", style: TTypography.label, color: colors.textDefault)
              Spacer().frame(height: 6)
              TText("잠시 후 다시 시도해주세요.", style: TTypography.caption, color: colors.textMuted)
              Spacer().frame(height: 20)
              TButton("다시 시도", variant: .secondary) { model.refetch() }
            }
            .frame(maxWidth: .infinity)
            .padding(.vertical, 24)
          } else {
            if !model.sites.isEmpty {
              HomeCard {
                ForEach(model.sites) { site in
                  Button {
                    model.select(site.id)
                    onSelected()
                  } label: {
                    row(site, isCurrent: site.id == model.current?.id)
                  }
                  .buttonStyle(.plain)
                }
              }
            }
            if model.isSettled {
              HomeCard {
                Button(action: onCreate) { createRow }
                  .buttonStyle(.plain)
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
        TImage(url: site.logo, side: 36)
          .clipShape(TShapes.squircle(10))
          .accessibilityHidden(true)
        VStack(alignment: .leading, spacing: 2) {
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

    private var createRow: some View {
      HStack(spacing: 12) {
        TIcon(LucideIcon.plus, size: 18, tint: colors.textMuted)
          .frame(width: 36, height: 36)
        TText("새 스페이스 생성", style: TTypography.label, color: colors.textMuted)
        Spacer(minLength: 0)
      }
      .padding(.vertical, 10)
      .contentShape(Rectangle())
      .accessibilityElement(children: .combine)
    }
  }

#endif
