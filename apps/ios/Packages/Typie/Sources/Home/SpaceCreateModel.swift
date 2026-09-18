import Design
import Observation

@MainActor @Observable
public final class SpaceCreateModel {
  public let form: TFormState
  public let name: TFieldState
  public private(set) var isSubmitting = false

  @ObservationIgnored private let create: @MainActor (String) async -> Bool

  public init(create: @escaping @MainActor (String) async -> Bool) {
    self.create = create
    let form = TFormState(autoFocusFirstField: false, validatesOnBlur: false)
    name = form.field(rules: [])
    self.form = form
  }

  public func focusName() {
    name.requestFocus()
  }

  public func submit() async -> Bool? {
    guard !isSubmitting else { return nil }
    isSubmitting = true
    defer { isSubmitting = false }
    form.endEditing()
    return await create(name.value)
  }
}
