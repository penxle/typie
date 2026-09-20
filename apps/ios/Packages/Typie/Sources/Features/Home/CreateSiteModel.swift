import Design
import FactoryKit
import Observation

@MainActor @Observable
public final class CreateSiteModel {
  public let form: TFormState
  public let name: TFieldState
  public private(set) var isSubmitting = false

  @ObservationIgnored private let sites = Container.shared.sites()

  public init() {
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
    return await sites.create(name: name.value)
  }
}
