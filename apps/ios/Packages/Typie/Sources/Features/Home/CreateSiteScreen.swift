#if canImport(UIKit)

  import Design
  import FactoryKit
  import SwiftUI

  @MainActor
  struct CreateSiteScreen: View {
    @Environment(\.theme) private var theme
    private var colors: TColors { theme.colors }

    @State private var keyboardVisible = false

    private let model: CreateSiteModel
    private let toast = Container.shared.toast()

    init(model: CreateSiteModel) {
      self.model = model
    }

    var body: some View {
      GeometryReader { proxy in
        VStack(spacing: 0) {
          ScrollView {
            VStack(alignment: .leading, spacing: 0) {
              Spacer().frame(height: 8)
              TText(
                "스페이스는 독립된 글쓰기 공간이에요.\n주제나 목적에 따라 글을 나누어 관리해보세요.",
                style: TTypography.text, color: colors.textMuted)
              Spacer().frame(height: 24)
              TTextField(
                model.name, form: model.form, label: "스페이스 이름", placeholder: "새 스페이스",
                onSubmit: { Task { await submit() } })
              Spacer().frame(height: 12)
            }
            .frame(maxWidth: .infinity, alignment: .leading)
          }
          .scrollBounceBehavior(.basedOnSize)
          .onTapGesture { model.form.endEditing() }
          TButton("생성", loading: model.isSubmitting, height: 56, textStyle: TTypography.title) {
            await submit()
          }
        }
        .padding(.horizontal, 16)
        .padding(.bottom, keyboardVisible ? 12 : max(0, 16 - proxy.safeAreaInsets.bottom))
        .frame(maxWidth: 600)
        .frame(maxWidth: .infinity, maxHeight: .infinity)
      }
      .sheetBackground()
      .observingKeyboard($keyboardVisible)
    }

    private func submit() async {
      switch await model.submit() {
      case true?:
        toast.success("새 스페이스가 생성되었어요.")
      case false?:
        toast.error("오류가 발생했어요. 잠시 후 다시 시도해주세요.")
      case nil:
        break
      }
    }
  }

#endif
