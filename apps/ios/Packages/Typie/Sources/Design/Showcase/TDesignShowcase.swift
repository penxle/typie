#if canImport(UIKit)

  import SwiftUI

  public struct TDesignShowcase: View {
    @Environment(\.theme) private var theme
    private var colors: TColors { theme.colors }
    private var shadows: TShadows { theme.shadows }

    @Bindable private var settings: TThemeSettings
    @State private var loading = false
    @State private var sample = ShowcaseForm()
    @State private var toast = TToastCenter()
    @State private var dialog = TDialogCenter()
    @State private var typeSize: DynamicTypeSize = .large

    public init(theme: TThemeSettings) {
      settings = theme
    }

    public var body: some View {
      ScrollView {
        VStack(alignment: .leading, spacing: 24) {
          TLogo(height: 32)

          Picker("테마", selection: $settings.mode) {
            Text("시스템").tag(TThemeMode.system)
            Text("라이트").tag(TThemeMode.light)
            Text("다크").tag(TThemeMode.dark)
          }
          .pickerStyle(.segmented)

          section("타이포그래피") {
            Picker("글자 크기", selection: $typeSize) {
              Text("기본").tag(DynamicTypeSize.large)
              Text("큼").tag(DynamicTypeSize.xxxLarge)
              Text("최대").tag(DynamicTypeSize.accessibility5)
            }
            .pickerStyle(.segmented)
            VStack(alignment: .leading, spacing: 12) {
              TText("화면 제목 hero 28/36", style: TTypography.hero)
              TText("섹션 제목 heading 22/28", style: TTypography.heading)
              TText("내비·대화상자 제목 title 17/22", style: TTypography.title)
              TText("본문 text 16/24 — 언제든 이어 쓰는 글쓰기 앱, 타이피.", style: TTypography.text)
              TText("행 제목 label 15/20", style: TTypography.label)
              TText("버튼·입력 control 15/20", style: TTypography.control)
              TText("보조 본문 detail 14/20", style: TTypography.detail, color: colors.textMuted)
              TText("설명 caption 13/18", style: TTypography.caption, color: colors.textMuted)
              TText("섹션 헤더 section 13/18", style: TTypography.section, color: colors.textMuted)
              TText("경로·시각 meta 12/16", style: TTypography.meta, color: colors.textHint)
              TText("극소 fine 11/16", style: TTypography.fine, color: colors.textHint)
              HStack(spacing: 8) {
                TIcon(LucideIcon.search, size: 16, relativeTo: TTypography.control)
                TText("아이콘은 동반 텍스트를 따라 배율", style: TTypography.control)
              }
            }
            .dynamicTypeSize(typeSize)
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

          section("링") {
            HStack(spacing: 16) {
              TProgressRing(progress: 0.35, state: .under, size: 16)
              TProgressRing(progress: 0.35, state: .under, size: 72)
              TProgressRing(progress: 0.35, state: .achieved, size: 72)
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

          section("입력") {
            TTextField(
              sample.email, form: sample.form, label: "이메일", placeholder: "me@example.com",
              contentType: .username, keyboardType: .emailAddress, showsSuccess: true)
            TTextField(
              sample.password, form: sample.form, label: "비밀번호", placeholder: "********",
              contentType: .password, isSecure: true)
            TButton("기본 버튼") { _ = sample.form.validate() }
          }
          .animation(.smooth(duration: 0.22), value: [sample.email.error, sample.password.error])

          section("대화상자") {
            TButton("보조 버튼", variant: .secondary) {
              dialog.present(
                TDialogItem(
                  title: "잘못된 이메일 또는 비밀번호예요",
                  message: "입력한 로그인 정보가 일치하지 않아요. 이메일과 비밀번호를 다시 한번 확인해주세요.",
                  confirmText: "확인"))
            }

            TButton("확인 대화상자", variant: .secondary) {
              _ = await dialog.confirm(
                TDialogItem(
                  title: "일일 목표를 해제하시겠어요?", message: "설정한 하루 목표 글자 수가 사라져요.",
                  confirmText: "해제", cancelText: "취소", confirmIsDestructive: true))
            }
          }

          section("토스트") {
            TButton("위험 버튼", variant: .danger) {
              toast.error("오류가 발생했어요. 잠시 후 다시 시도해주세요.")
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
      .canvasBackground()
      .overlay { TToastOverlay(center: toast) }
      .overlay { TDialogOverlay(center: dialog) }
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

  @MainActor
  private final class ShowcaseForm {
    let form = TFormState(autoFocusFirstField: false, validatesOnBlur: true)
    let email: TFieldState
    let password: TFieldState

    init() {
      email = form.field(rules: [
        .required("이메일을 입력해주세요."), .email("올바른 이메일 형식을 입력해주세요."),
      ])
      password = form.field(rules: [.required("비밀번호를 입력해주세요.")])
    }
  }

#endif
