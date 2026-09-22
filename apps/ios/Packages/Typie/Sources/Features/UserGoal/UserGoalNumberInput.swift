#if canImport(UIKit)

  import Design
  import SwiftUI
  import UIKit

  struct UserGoalNumberInput: UIViewRepresentable {
    let value: Int
    let style: TTextStyle
    let color: Color
    @Binding var focused: Bool
    let onChange: (Int) -> Void
    let onEnd: () -> Void

    func makeCoordinator() -> Coordinator {
      Coordinator(onChange: onChange, onEnd: onEnd)
    }

    func makeUIView(context: Context) -> UITextField {
      let view = UITextField()
      view.borderStyle = .none
      view.keyboardType = .numberPad
      view.textAlignment = .center
      view.adjustsFontForContentSizeCategory = true
      view.font = Self.tabularFont(style.uiFont(for: view.traitCollection))
      view.text = Coordinator.format(String(value))
      view.delegate = context.coordinator
      view.setContentHuggingPriority(.required, for: .horizontal)
      view.setContentCompressionResistancePriority(.required, for: .horizontal)
      return view
    }

    func updateUIView(_ uiView: UITextField, context: Context) {
      context.coordinator.onChange = onChange
      context.coordinator.onEnd = onEnd
      let font = Self.tabularFont(style.uiFont(for: uiView.traitCollection))
      if uiView.font != font { uiView.font = font }
      let uiColor = UIColor(color)
      if uiView.textColor != uiColor {
        uiView.textColor = uiColor
        uiView.tintColor = uiColor
      }
      if !uiView.isFirstResponder {
        let text = Coordinator.format(String(value))
        if uiView.text != text { uiView.text = text }
      }
      if focused, !uiView.isFirstResponder {
        Task { @MainActor in uiView.becomeFirstResponder() }
      } else if !focused, uiView.isFirstResponder {
        Task { @MainActor in uiView.resignFirstResponder() }
      }
    }

    func sizeThatFits(
      _ proposal: ProposedViewSize, uiView: UITextField, context: Context
    ) -> CGSize? {
      uiView.intrinsicContentSize
    }

    private static func tabularFont(_ font: UIFont) -> UIFont {
      let descriptor = font.fontDescriptor.addingAttributes([
        .featureSettings: [
          [
            UIFontDescriptor.FeatureKey.type: kNumberSpacingType,
            UIFontDescriptor.FeatureKey.selector: kMonospacedNumbersSelector,
          ]
        ]
      ])
      return UIFont(descriptor: descriptor, size: 0)
    }

    final class Coordinator: NSObject, UITextFieldDelegate {
      var onChange: (Int) -> Void
      var onEnd: () -> Void
      private var valueAtBegin: Int?

      init(onChange: @escaping (Int) -> Void, onEnd: @escaping () -> Void) {
        self.onChange = onChange
        self.onEnd = onEnd
      }

      static func format(_ digits: String) -> String {
        guard let number = Int(digits) else { return "" }
        return number.formatted(.number)
      }

      func textField(
        _ textField: UITextField, shouldChangeCharactersIn range: NSRange,
        replacementString string: String
      ) -> Bool {
        let current = (textField.text ?? "") as NSString
        let proposed = current.replacingCharacters(in: range, with: string) as NSString
        let caret = range.location + (string as NSString).length
        let digitsBeforeCaret = Self.digitCount(in: proposed.substring(to: caret))
        let digits = Self.digitsOnly(proposed as String)
        guard digits.count <= 10 else { return false }
        let formatted = Self.format(digits)
        textField.text = formatted
        var seen = 0
        var offset = 0
        if digitsBeforeCaret > 0 {
          for (index, character) in formatted.enumerated() where character.isNumber {
            seen += 1
            if seen == digitsBeforeCaret {
              offset = index + 1
              break
            }
          }
        }
        if let position = textField.position(from: textField.beginningOfDocument, offset: offset) {
          textField.selectedTextRange = textField.textRange(from: position, to: position)
        }
        if let number = Int(digits), number > 0 { onChange(number) }
        return false
      }

      func textFieldDidBeginEditing(_ textField: UITextField) {
        valueAtBegin = Int(Self.digitsOnly(textField.text ?? ""))
      }

      func textFieldDidEndEditing(_ textField: UITextField) {
        if Self.digitsOnly(textField.text ?? "").isEmpty, let restored = valueAtBegin {
          onChange(restored)
        }
        valueAtBegin = nil
        onEnd()
      }

      private static func digitsOnly(_ text: String) -> String {
        text.filter(\.isNumber)
      }

      private static func digitCount(in text: String) -> Int {
        text.reduce(0) { $0 + ($1.isNumber ? 1 : 0) }
      }
    }
  }

#endif
