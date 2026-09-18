import Observation

@Observable
@MainActor
public final class TFormState {
  public let autoFocusFirstField: Bool
  public let validatesOnBlur: Bool
  private var fields: [TFieldState] = []

  public init(autoFocusFirstField: Bool = true, validatesOnBlur: Bool = false) {
    self.autoFocusFirstField = autoFocusFirstField
    self.validatesOnBlur = validatesOnBlur
  }

  public func field(_ initial: String = "", rules: [TFormRule]) -> TFieldState {
    let field = TFieldState(value: initial, rules: rules, validatesOnBlur: validatesOnBlur)
    fields.append(field)
    return field
  }

  public func validate() -> Bool {
    for field in fields {
      field.validate()
    }
    guard let failed = fields.first(where: { !$0.errors.isEmpty }) else { return true }
    failed.requestFocus()
    return false
  }

  public func endEditing() {
    for field in fields where field.isFocused {
      field.resignFocus()
    }
  }

  public func isFirst(_ field: TFieldState) -> Bool {
    fields.first === field
  }

  public func isLast(_ field: TFieldState) -> Bool {
    fields.last === field
  }

  public func focusNext(after field: TFieldState) {
    guard let index = fields.firstIndex(where: { $0 === field }), index + 1 < fields.count else {
      return
    }
    fields[index + 1].requestFocus()
  }
}
