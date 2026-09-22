#if canImport(UIKit)

  import Design
  import FactoryKit
  import SwiftUI

  @MainActor
  struct UserGoalFormScreen: View {
    static let contentHeight: CGFloat = 16 + 52 + 24 + 56 + 16

    private static let valueStyle = TTextStyle(
      size: 44, weight: .semibold, lineHeight: 52, relativeTo: .largeTitle)
    private static let unitStyle = TTextStyle(
      size: 17, weight: .regular, lineHeight: 22, relativeTo: .body)
    private static let glyphStyle = TTextStyle(
      size: 26, weight: .regular, lineHeight: 32, relativeTo: .title)

    @Environment(\.theme) private var theme
    @Environment(\.accessibilityReduceMotion) private var reduceMotion
    private var colors: TColors { theme.colors }

    private let model: UserGoalFormModel
    private let onDone: () -> Void
    private let toast = Container.shared.toast()

    @State private var isEditing = false
    @State private var focused = false

    init(model: UserGoalFormModel, onDone: @escaping () -> Void) {
      self.model = model
      self.onDone = onDone
    }

    var body: some View {
      VStack(spacing: 0) {
        HStack(spacing: 24) {
          UserGoalStepButton(
            glyph: "−", enabled: model.canDecrement && !isEditing, style: Self.glyphStyle
          ) {
            large in
            model.adjust(by: large ? -UserGoalFormModel.largeStep : -UserGoalFormModel.step)
          }
          value
          UserGoalStepButton(
            glyph: "+", enabled: model.canIncrement && !isEditing, style: Self.glyphStyle
          ) {
            large in model.adjust(by: large ? UserGoalFormModel.largeStep : UserGoalFormModel.step)
          }
        }
        .padding(.top, 16)
        Spacer().frame(height: 24)
        TButton(
          "저장", enabled: model.canSubmit && !model.isRemoving, loading: model.isSubmitting,
          height: 56, textStyle: TTypography.title
        ) {
          await submit()
        }
        .padding(.bottom, 16)
      }
      .padding(.horizontal, 16)
      .frame(maxWidth: 600)
      .frame(maxWidth: .infinity, maxHeight: .infinity, alignment: .top)
      .background { Color.clear.contentShape(Rectangle()).onTapGesture { focused = false } }
      .sheetBackground()
    }

    private var value: some View {
      HStack(alignment: .firstTextBaseline, spacing: 4) {
        if isEditing {
          UserGoalNumberInput(
            value: model.value, style: Self.valueStyle, color: colors.textDefault,
            focused: $focused, onChange: { model.enter(String($0)) },
            onEnd: {
              focused = false
              isEditing = false
            }
          )
          .fixedSize()
          .accessibilityLabel("하루 글자 수")
        } else {
          Button {
            isEditing = true
            focused = true
          } label: {
            TText(
              model.value.formatted(.number), style: Self.valueStyle, color: colors.textDefault,
              monospacedDigit: true)
          }
          .buttonStyle(.plain)
          .accessibilityLabel("하루 글자 수")
          .accessibilityValue("\(model.value)자")
          .accessibilityHint("눌러서 직접 입력")
        }
        TText("자", style: Self.unitStyle, color: colors.textMuted)
      }
    }

    private func submit() async {
      focused = false
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
  }

  @MainActor
  private struct UserGoalStepButton: View {
    private static let side: CGFloat = 52
    private static let holdDelay: TimeInterval = 0.42
    private static let repeatInterval: TimeInterval = 0.14
    private static let fastInterval: TimeInterval = 0.09
    private static let fastAfter = 6

    @Environment(\.theme) private var theme
    @Environment(\.accessibilityReduceMotion) private var reduceMotion

    let glyph: String
    let enabled: Bool
    let style: TTextStyle
    let step: (_ large: Bool) -> Void

    @State private var pressed = false
    @State private var repeater: Task<Void, Never>?

    var body: some View {
      TText(glyph, style: style, color: theme.colors.textDefault)
        .frame(width: Self.side, height: Self.side)
        .background(pressed ? theme.colors.surfaceActive : theme.colors.surfaceInset, in: Circle())
        .scaleEffect(pressed && !reduceMotion ? 0.94 : 1)
        .opacity(enabled ? 1 : 0.35)
        .contentShape(Circle())
        .animation(reduceMotion ? nil : .easeOut(duration: 0.1), value: pressed)
        .onLongPressGesture(minimumDuration: .infinity, maximumDistance: 24) {
        } onPressingChanged: { pressing in
          pressing ? begin() : end()
        }
        .accessibilityLabel(glyph == "+" ? "늘리기" : "줄이기")
        .accessibilityAddTraits(.isButton)
    }

    private func begin() {
      guard enabled else { return }
      pressed = true
      step(false)
      repeater?.cancel()
      repeater = Task { @MainActor in
        var count = 0
        try? await Task.sleep(for: .seconds(Self.holdDelay))
        while !Task.isCancelled {
          count += 1
          step(count > Self.fastAfter)
          try? await Task.sleep(
            for: .seconds(count > Self.fastAfter ? Self.fastInterval : Self.repeatInterval))
        }
      }
    }

    private func end() {
      pressed = false
      repeater?.cancel()
      repeater = nil
    }
  }

#endif
