import SwiftUI

public struct DesignShowcase: View {
  @Environment(\.theme) private var theme
  private var colors: TColors { theme.colors }
  private var shadows: TShadows { theme.shadows }

  @Bindable private var settings: ThemeSettings
  @State private var loading = false

  public init(theme: ThemeSettings) {
    settings = theme
  }

  public var body: some View {
    ScrollView {
      VStack(alignment: .leading, spacing: 24) {
        TLogo(height: 32)

        Picker("테마", selection: $settings.mode) {
          Text("시스템").tag(ThemeMode.system)
          Text("라이트").tag(ThemeMode.light)
          Text("다크").tag(ThemeMode.dark)
        }
        .pickerStyle(.segmented)

        section("타이포그래피") {
          TText("화면 대제목 display 28/36", style: TTypography.display)
          TText("섹션 제목 heading 22/28", style: TTypography.heading)
          TText("카드 제목 title 17/22", style: TTypography.title)
          TText("메뉴 항목 label 15/20", style: TTypography.label)
          TText("본문 body 16/24 — 언제든 이어 쓰는 글쓰기 앱, 타이피.", style: TTypography.body)
          TText("버튼 action 15/20", style: TTypography.action)
          TText("헬퍼 caption 13/18", style: TTypography.caption, color: colors.textMuted)
          TText("배지 micro 11/16", style: TTypography.micro, color: colors.textHint)
        }

        section("색") {
          swatch("surfaceCanvas", colors.surfaceCanvas)
          swatch("surfaceDefault", colors.surfaceDefault)
          swatch("surfaceInset", colors.surfaceInset)
          swatch("surfaceInverse", colors.surfaceInverse)
          swatch("borderDefault", colors.borderDefault)
          swatch("danger", colors.dangerDefault)
          swatch("success", colors.successDefault)
          HStack(spacing: 8) {
            ForEach(
              [
                colors.paletteGray, colors.paletteRed, colors.paletteOrange,
                colors.paletteYellow, colors.paletteGreen, colors.paletteBlue,
                colors.palettePurple,
              ], id: \.self
            ) { color in
              TShapes.squircle(TShapes.sm).fill(color).frame(width: 28, height: 28)
            }
          }
        }

        section("버튼") {
          TButton("기본 버튼") {}
          TButton("보조 버튼", variant: .secondary, leadingIcon: LucideIcon.headphones) {}
          TButton("위험 버튼", variant: .danger) {}
          TButton("비활성", enabled: false) {}
          TButton("로딩 토글", loading: loading, loadingText: "처리 중") {
            loading = true
            try? await Task.sleep(for: .seconds(2))
            loading = false
          }
        }

        section("아이콘") {
          HStack(spacing: 16) {
            TIcon(LucideIcon.aArrowDown)
            TIcon(LucideIcon.construction)
            TIcon(LucideIcon.circleFadingArrowUp)
            TIcon(LucideIcon.headphones)
            TIcon(LucideIcon.settings, tint: colors.textMuted)
            TIcon(TypieIcon.bellFilled)
            TIcon(TypieIcon.folderFilled, tint: colors.paletteBlue)
          }
        }

        section("그림자·셰이프") {
          HStack(spacing: 16) {
            card(shadows.sm, "sm")
            card(shadows.md, "md")
            card(shadows.lg, "lg")
            card(shadows.xl, "xl")
          }
        }
      }
      .padding(16)
    }
    .background(colors.surfaceCanvas)
  }

  private func section(_ title: String, @ViewBuilder content: () -> some View) -> some View {
    VStack(alignment: .leading, spacing: 12) {
      TText(title, style: TTypography.label, color: colors.textMuted)
      content()
    }
  }

  private func swatch(_ name: String, _ color: Color) -> some View {
    HStack(spacing: 12) {
      TShapes.rounded(TShapes.sm).fill(color)
        .overlay(TShapes.rounded(TShapes.sm).stroke(colors.borderDefault))
        .frame(width: 28, height: 28)
      TText(name, style: TTypography.caption)
    }
  }

  private func card(_ shadow: TShadow, _ name: String) -> some View {
    TText(name, style: TTypography.caption)
      .frame(width: 64, height: 64)
      .background(colors.surfaceDefault.shadow(shadow), in: TShapes.squircle(TShapes.md))
  }
}
