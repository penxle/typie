#if canImport(UIKit)

  import Design
  import FactoryKit
  import SwiftUI

  @MainActor
  struct UserGoalFormScreen: View {
    @State private var keyboardVisible = false

    private let model: UserGoalFormModel
    private let onDone: () -> Void
    private let toast = Container.shared.toast()
    private let dialog = Container.shared.dialog()

    init(model: UserGoalFormModel, onDone: @escaping () -> Void) {
      self.model = model
      self.onDone = onDone
    }

    var body: some View {
      GeometryReader { proxy in
        VStack(spacing: 0) {
          ScrollView {
            VStack(alignment: .leading, spacing: 0) {
              Spacer().frame(height: 8)
              TTextField(
                model.target, form: model.form, label: "하루 글자 수", placeholder: "",
                keyboardType: .numberPad, onSubmit: { Task { await submit() } })
              Spacer().frame(height: 12)
            }
            .frame(maxWidth: .infinity, alignment: .leading)
          }
          .scrollBounceBehavior(.basedOnSize)
          .onTapGesture { model.form.endEditing() }
          VStack(spacing: 12) {
            TButton(
              "저장", enabled: !model.isRemoving, loading: model.isSubmitting, height: 56,
              textStyle: TTypography.title
            ) {
              await submit()
            }
            if model.hasGoal {
              TButton(
                "해제", variant: .danger, enabled: !model.isSubmitting, loading: model.isRemoving
              ) { await remove() }
            }
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
        toast.success("일일 목표를 저장했어요.")
        onDone()
      case false?:
        toast.error("오류가 발생했어요. 잠시 후 다시 시도해주세요.")
      case nil:
        break
      }
    }

    private func remove() async {
      model.form.endEditing()
      let confirmed = await dialog.confirm(
        TDialogItem(
          title: "일일 목표를 해제하시겠어요?", message: "설정한 하루 목표 글자 수가 사라져요.",
          confirmText: "해제", cancelText: "취소", confirmIsDestructive: true))
      guard confirmed else { return }
      if await model.remove() {
        toast.success("일일 목표를 해제했어요.")
        onDone()
      } else {
        toast.error("오류가 발생했어요. 잠시 후 다시 시도해주세요.")
      }
    }
  }

#endif
