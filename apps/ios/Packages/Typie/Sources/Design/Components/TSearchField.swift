#if canImport(UIKit)

  import SwiftUI

  public struct TSearchField: View {
    public static let clearButtonWidth: CGFloat = 40

    @Environment(\.theme) private var theme
    @Environment(\.accessibilityReduceMotion) private var reduceMotion
    private var colors: TColors { theme.colors }

    @Binding private var text: String
    private var isFocused: FocusState<Bool>.Binding
    private let placeholder: String
    private let onSubmit: () -> Void

    public init(
      text: Binding<String>, isFocused: FocusState<Bool>.Binding, placeholder: String,
      onSubmit: @escaping () -> Void
    ) {
      _text = text
      self.isFocused = isFocused
      self.placeholder = placeholder
      self.onSubmit = onSubmit
    }

    public var body: some View {
      HStack(spacing: 8) {
        TextField("", text: $text, prompt: Text(placeholder).foregroundStyle(colors.textHint))
          .font(TTypography.control.font)
          .foregroundStyle(colors.textDefault)
          .tint(colors.textDefault)
          .focused(isFocused)
          .submitLabel(.search)
          .autocorrectionDisabled()
          .textInputAutocapitalization(.never)
          .onSubmit(onSubmit)
          .accessibilityLabel("검색")
          .padding(.vertical, 10)
        if !text.isEmpty {
          Button {
            text = ""
          } label: {
            TIcon(
              LucideIcon.circleX, size: 18, tint: colors.textHint, relativeTo: TTypography.control
            )
            .frame(minWidth: Self.clearButtonWidth, minHeight: 44)
            .contentShape(Rectangle())
          }
          .buttonStyle(.plain)
          .accessibilityLabel("지우기")
          .transition(.opacity)
        }
      }
      .frame(maxHeight: .infinity)
      .contentShape(Rectangle())
      .onTapGesture { isFocused.wrappedValue = true }
      .animation(reduceMotion ? nil : .easeOut(duration: 0.15), value: text.isEmpty)
    }
  }

#endif
