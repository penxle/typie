import Observation

@Observable
@MainActor
public final class TFieldState: Identifiable {
  private var storedValue: String
  private var hasValidated = false

  public var value: String {
    get { storedValue }
    set {
      storedValue = newValue
      guard hasValidated else { return }
      revalidate()
    }
  }

  public internal(set) var errors: [String] = []
  public internal(set) var isFocused = false
  public private(set) var isVerified = false
  public private(set) var focusRequest = 0
  public private(set) var resignRequest = 0

  let rules: [TFormRule]
  let validatesOnBlur: Bool

  init(value: String, rules: [TFormRule], validatesOnBlur: Bool) {
    storedValue = value
    self.rules = rules
    self.validatesOnBlur = validatesOnBlur
  }

  public var error: String? { errors.first }

  public func requestFocus() {
    focusRequest += 1
  }

  public func resignFocus() {
    resignRequest += 1
  }

  func validate() {
    hasValidated = true
    revalidate()
  }

  func editingEnded() {
    isFocused = false
    guard validatesOnBlur, !hasValidated, !isBlank else { return }
    validate()
  }

  private func revalidate() {
    errors = rules.compactMap { $0.validate(storedValue) }
    isVerified = errors.isEmpty && !isBlank
  }

  private var isBlank: Bool {
    storedValue.allSatisfy(\.isWhitespace)
  }
}
