import Design
import FactoryKit
import Observation

@MainActor @Observable
final class CreateSiteModel {
  let form: TFormState
  let name: TFieldState
  private(set) var isSubmitting = false

  @ObservationIgnored private let sites = Container.shared.sites()

  init() {
    let form = TFormState(autoFocusFirstField: false, validatesOnBlur: false)
    name = form.field(rules: [])
    self.form = form
  }

  func focusName() {
    name.requestFocus()
  }

  func submit() async -> Bool? {
    guard !isSubmitting else { return nil }
    isSubmitting = true
    defer { isSubmitting = false }
    form.endEditing()
    return await sites.create(name: name.value)
  }
}
