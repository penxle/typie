#if canImport(UIKit)

  import SwiftUI

  public struct TSearchField: View {
    public static var height: CGFloat { SearchFieldChrome.height }

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
        TIcon(
          LucideIcon.search, size: 16, tint: colors.textHint, relativeTo: TTypography.control)
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
            .frame(minWidth: 40, minHeight: 44)
            .contentShape(Rectangle())
          }
          .buttonStyle(.plain)
          .accessibilityLabel("지우기")
          .transition(.opacity)
        }
      }
      .padding(.leading, SearchFieldChrome.horizontalPadding)
      .padding(.trailing, text.isEmpty ? SearchFieldChrome.horizontalPadding : 0)
      .modifier(SearchFieldChrome())
      .onTapGesture { isFocused.wrappedValue = true }
      .animation(reduceMotion ? nil : .easeOut(duration: 0.15), value: text.isEmpty)
    }
  }

  public struct TSearchFieldButton: View {
    @Environment(\.theme) private var theme
    private var colors: TColors { theme.colors }

    private let placeholder: String
    private let action: () -> Void

    public init(placeholder: String, action: @escaping () -> Void) {
      self.placeholder = placeholder
      self.action = action
    }

    public var body: some View {
      Button(action: action) {
        HStack(spacing: 8) {
          TIcon(
            LucideIcon.search, size: 16, tint: colors.textHint, relativeTo: TTypography.control)
          TText(placeholder, style: TTypography.control, color: colors.textHint, maxLines: 1)
            .padding(.vertical, 12)
          Spacer(minLength: 0)
        }
        .padding(.horizontal, SearchFieldChrome.horizontalPadding)
        .modifier(SearchFieldChrome())
      }
      .buttonStyle(.plain)
      .accessibilityLabel("검색")
    }
  }

  struct SearchFieldChrome: ViewModifier {
    @Environment(\.theme) private var theme

    static let height: CGFloat = 44
    static let horizontalPadding: CGFloat = 12

    func body(content: Content) -> some View {
      content
        .frame(minHeight: Self.height)
        .background(theme.colors.surfaceInset, in: TShapes.rounded(TShapes.md))
        .contentShape(TShapes.rounded(TShapes.md))
    }
  }

#endif
