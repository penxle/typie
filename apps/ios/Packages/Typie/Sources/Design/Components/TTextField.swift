#if canImport(UIKit)

  import SwiftUI
  import UIKit

  public struct TTextField: View {
    @Environment(\.theme) private var theme
    @Environment(\.accessibilityReduceMotion) private var reduceMotion
    @Environment(\.scenePhase) private var scenePhase
    private var colors: TColors { theme.colors }

    @State private var revealed = false
    @ScaledMetric(relativeTo: TTypography.text.textStyle) private var statusBadgeSide: CGFloat = 18

    private let field: TFieldState
    private let form: TFormState
    private let label: String
    private let placeholder: String
    private let contentType: UITextContentType?
    private let keyboardType: UIKeyboardType
    private let isSecure: Bool
    private let showsSuccess: Bool
    private let onSubmit: (() -> Void)?

    public init(
      _ field: TFieldState,
      form: TFormState,
      label: String,
      placeholder: String,
      contentType: UITextContentType? = nil,
      keyboardType: UIKeyboardType = .default,
      isSecure: Bool = false,
      showsSuccess: Bool = false,
      onSubmit: (() -> Void)? = nil
    ) {
      self.field = field
      self.form = form
      self.label = label
      self.placeholder = placeholder
      self.contentType = contentType
      self.keyboardType = keyboardType
      self.isSecure = isSecure
      self.showsSuccess = showsSuccess
      self.onSubmit = onSubmit
    }

    private var borderColor: Color {
      if field.error != nil {
        colors.dangerDefault
      } else if field.isFocused {
        colors.borderEmphasis
      } else {
        colors.borderHairline
      }
    }

    private var borderWidth: CGFloat {
      field.error != nil || field.isFocused ? 1.5 : 1
    }

    private var animation: Animation {
      reduceMotion ? .linear(duration: 0.15) : .smooth(duration: 0.22)
    }

    private var showsVerified: Bool { showsSuccess && field.isVerified && field.error == nil }
    private var hasStatus: Bool { field.error != nil || showsVerified }
    private var showsClear: Bool { field.isFocused && !field.value.isEmpty }
    private var hasAccessory: Bool { isSecure || showsClear || hasStatus }

    private func statusBadge(_ color: Color, @ViewBuilder glyph: () -> some View) -> some View {
      ZStack {
        Circle().fill(color).frame(width: statusBadgeSide, height: statusBadgeSide)
        glyph()
      }
    }

    public var body: some View {
      let shape = TShapes.rounded(TShapes.md)
      VStack(alignment: .leading, spacing: 0) {
        TText(label, style: TTypography.caption, color: colors.textMuted)
        Spacer().frame(height: 8)
        HStack(spacing: 0) {
          TTextFieldInput(
            field: field, form: form, colors: colors, placeholder: placeholder,
            value: field.value, focusRequest: field.focusRequest,
            resignRequest: field.resignRequest,
            contentType: contentType ?? (isSecure ? .password : nil),
            keyboardType: isSecure && keyboardType == .default ? .asciiCapable : keyboardType,
            isSecure: isSecure && !revealed, onSubmit: onSubmit
          )
          .padding(.vertical, 12)
          if hasAccessory {
            Spacer().frame(width: 8)
          }
          if isSecure {
            Button {
              revealed.toggle()
            } label: {
              TIcon(
                revealed ? LucideIcon.eyeOff : LucideIcon.eye, size: 18, tint: colors.textHint,
                relativeTo: TTypography.text
              )
              .contentTransition(.identity)
              .frame(minWidth: 36, minHeight: 44)
              .contentShape(Rectangle())
            }
            .buttonStyle(.plain)
            .accessibilityLabel(revealed ? "비밀번호 가리기" : "비밀번호 보기")
          }
          if showsClear {
            Button {
              field.value = ""
            } label: {
              TIcon(
                LucideIcon.circleX, size: 18, tint: colors.textHint, relativeTo: TTypography.text
              )
              .frame(minWidth: 36, minHeight: 44)
              .contentShape(Rectangle())
            }
            .buttonStyle(.plain)
            .accessibilityLabel("지우기")
            .transition(.opacity)
          }
          if field.error != nil {
            statusBadge(colors.dangerDefault) {
              TIcon(
                TypieIcon.exclamation, size: 10, tint: colors.textOnDanger, label: "오류",
                relativeTo: TTypography.text)
            }
            .frame(minWidth: 36, minHeight: 44)
            .contentShape(Rectangle())
            .allowsHitTesting(false)
            .transition(.move(edge: .trailing).combined(with: .opacity))
          } else if showsVerified {
            statusBadge(colors.successDefault) {
              TIcon(
                LucideIcon.check, size: 11, tint: colors.textOnSuccess, label: "확인됨",
                relativeTo: TTypography.text)
            }
            .frame(minWidth: 36, minHeight: 44)
            .contentShape(Rectangle())
            .allowsHitTesting(false)
            .transition(.move(edge: .trailing).combined(with: .opacity))
          }
        }
        .padding(.leading, 16)
        .padding(.trailing, hasAccessory ? 7 : 16)
        .frame(maxWidth: .infinity, minHeight: 48)
        .background(colors.surfaceDefault, in: shape)
        .clipShape(shape)
        .overlay(shape.strokeBorder(borderColor, lineWidth: borderWidth))
        .contentShape(shape)
        .onTapGesture { field.requestFocus() }
        if let error = field.error {
          VStack(alignment: .leading, spacing: 0) {
            Spacer().frame(height: 4)
            TText(error, style: TTypography.caption, color: colors.dangerDefault)
              .padding(.leading, 12)
          }
          .transition(.asymmetric(insertion: .opacity, removal: .identity))
        }
      }
      .animation(animation, value: field.isFocused)
      .animation(animation, value: field.error)
      .animation(animation, value: showsVerified)
      .animation(animation, value: showsClear)
      .onChange(of: scenePhase) { _, phase in
        if phase != .active { revealed = false }
      }
      .onAppear {
        if form.autoFocusFirstField, form.isFirst(field) {
          field.requestFocus()
        }
      }
    }
  }

  private struct TTextFieldInput: UIViewRepresentable {
    let field: TFieldState
    let form: TFormState
    let colors: TColors
    let placeholder: String
    let value: String
    let focusRequest: Int
    let resignRequest: Int
    let contentType: UITextContentType?
    let keyboardType: UIKeyboardType
    let isSecure: Bool
    let onSubmit: (() -> Void)?

    @Environment(\.dynamicTypeSize) private var dynamicTypeSize

    func makeCoordinator() -> TTextFieldCoordinator {
      TTextFieldCoordinator(field: field, form: form, onSubmit: onSubmit)
    }

    func makeUIView(context: Context) -> UITextField {
      let view = UITextField()
      view.borderStyle = .none
      view.autocapitalizationType = .none
      view.autocorrectionType = .no
      view.spellCheckingType = .no
      view.isSecureTextEntry = isSecure
      view.textContentType = contentType
      view.keyboardType = keyboardType
      view.returnKeyType = form.isLast(field) ? .done : .next
      view.font = TTypography.text.uiFont(for: view.traitCollection)
      view.adjustsFontForContentSizeCategory = true
      view.text = value
      view.delegate = context.coordinator
      view.setContentHuggingPriority(.defaultLow, for: .horizontal)
      view.setContentCompressionResistancePriority(.defaultLow, for: .horizontal)
      view.addTarget(
        context.coordinator, action: #selector(TTextFieldCoordinator.editingChanged(_:)),
        for: .editingChanged)
      view.addTarget(
        context.coordinator, action: #selector(TTextFieldCoordinator.editingDidBegin),
        for: .editingDidBegin)
      view.addTarget(
        context.coordinator, action: #selector(TTextFieldCoordinator.editingDidEnd),
        for: .editingDidEnd)
      return view
    }

    func updateUIView(_ uiView: UITextField, context: Context) {
      let coordinator = context.coordinator
      coordinator.onSubmit = onSubmit
      uiView.returnKeyType = form.isLast(field) ? .done : .next
      let textColor = UIColor(colors.textDefault)
      if uiView.textColor != textColor {
        uiView.textColor = textColor
        uiView.tintColor = textColor
      }
      let hintColor = UIColor(colors.textHint)
      let font = TTypography.text.uiFont(for: uiView.traitCollection)
      if uiView.font != font { uiView.font = font }
      if uiView.attributedPlaceholder?.string != placeholder || coordinator.hintColor != hintColor
        || coordinator.placeholderFont != font
      {
        coordinator.hintColor = hintColor
        coordinator.placeholderFont = font
        uiView.attributedPlaceholder = NSAttributedString(
          string: placeholder, attributes: [.font: font, .foregroundColor: hintColor])
      }
      if uiView.markedTextRange == nil, uiView.text != value {
        uiView.text = value
      }
      if uiView.isSecureTextEntry != isSecure, uiView.markedTextRange == nil {
        let selection = uiView.selectedTextRange
        uiView.isSecureTextEntry = isSecure
        if uiView.isFirstResponder, let text = uiView.text, !text.isEmpty {
          uiView.text = ""
          uiView.insertText(text)
          uiView.selectedTextRange = selection
        }
      }
      if coordinator.focusRequest != focusRequest {
        let requested = focusRequest
        Task { @MainActor in
          if uiView.becomeFirstResponder() { coordinator.focusRequest = requested }
        }
      }
      if coordinator.resignRequest != resignRequest {
        coordinator.resignRequest = resignRequest
        Task { @MainActor in
          if uiView.isFirstResponder { uiView.resignFirstResponder() }
        }
      }
    }

    func sizeThatFits(
      _ proposal: ProposedViewSize, uiView: UITextField, context: Context
    ) -> CGSize? {
      let intrinsic = uiView.intrinsicContentSize
      guard let width = proposal.width, width.isFinite else { return intrinsic }
      return CGSize(width: width, height: intrinsic.height)
    }
  }

  private final class TTextFieldCoordinator: NSObject, UITextFieldDelegate {
    private let field: TFieldState
    private let form: TFormState
    var onSubmit: (() -> Void)?
    var focusRequest = 0
    var resignRequest = 0
    var hintColor: UIColor?
    var placeholderFont: UIFont?

    init(field: TFieldState, form: TFormState, onSubmit: (() -> Void)?) {
      self.field = field
      self.form = form
      self.onSubmit = onSubmit
    }

    @objc func editingChanged(_ textField: UITextField) {
      let text = textField.text ?? ""
      if field.value != text { field.value = text }
    }

    @objc func editingDidBegin() {
      field.isFocused = true
    }

    @objc func editingDidEnd() {
      field.editingEnded()
    }

    func textFieldShouldReturn(_ textField: UITextField) -> Bool {
      guard form.isLast(field) else {
        form.focusNext(after: field)
        return false
      }
      guard let onSubmit else { return true }
      onSubmit()
      return false
    }
  }

#endif
