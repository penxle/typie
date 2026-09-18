import Testing

@testable import Design

@MainActor
@Suite struct FormTests {
  nonisolated private static let emailRequired = "이메일을 입력해주세요."
  nonisolated private static let emailInvalid = "올바른 이메일 형식을 입력해주세요."
  nonisolated private static let passwordRequired = "비밀번호를 입력해주세요."

  @Test func requiredRejectsBlankValues() {
    let rule = TFormRule.required(Self.emailRequired)
    #expect(rule.validate("") == Self.emailRequired)
    #expect(rule.validate("   ") == Self.emailRequired)
    #expect(rule.validate("\n\t") == Self.emailRequired)
    #expect(rule.validate("a") == nil)
    #expect(rule.validate(" a ") == nil)
  }

  @Test func emailAcceptsWellFormedAddresses() {
    let rule = TFormRule.email(Self.emailInvalid)
    #expect(rule.validate("a@b.co") == nil)
    #expect(rule.validate("a+b.c_d-e@sub.example.com") == nil)
  }

  @Test func emailRejectsMalformedAddresses() {
    let rule = TFormRule.email(Self.emailInvalid)
    #expect(rule.validate("a@b") == Self.emailInvalid)
    #expect(rule.validate("a b@c.com") == Self.emailInvalid)
    #expect(rule.validate("@b.co") == Self.emailInvalid)
  }

  @Test func emailPassesBlankValues() {
    let rule = TFormRule.email(Self.emailInvalid)
    #expect(rule.validate("") == nil)
    #expect(rule.validate("  ") == nil)
  }

  @Test func validateSucceedsWithoutErrors() {
    let form = TFormState()
    let email = form.field(rules: [.required(Self.emailRequired), .email(Self.emailInvalid)])
    let password = form.field(rules: [.required(Self.passwordRequired)])
    email.value = "me@example.com"
    password.value = "secret"

    #expect(form.validate())
    #expect(email.errors.isEmpty)
    #expect(password.errors.isEmpty)
    #expect(email.focusRequest == 0)
    #expect(password.focusRequest == 0)
  }

  @Test func validateCollectsErrorsInRuleOrder() {
    let form = TFormState()
    let field = form.field(rules: [
      TFormRule { _ in Self.emailRequired },
      TFormRule { _ in Self.emailInvalid },
    ])

    #expect(form.validate() == false)
    #expect(field.errors == [Self.emailRequired, Self.emailInvalid])
    #expect(field.error == Self.emailRequired)
  }

  @Test func validateFocusesOnlyFirstFailingField() {
    let form = TFormState()
    let email = form.field(rules: [.required(Self.emailRequired), .email(Self.emailInvalid)])
    let password = form.field(rules: [.required(Self.passwordRequired)])

    #expect(form.validate() == false)
    #expect(email.errors == [Self.emailRequired])
    #expect(password.errors == [Self.passwordRequired])
    #expect(email.focusRequest == 1)
    #expect(password.focusRequest == 0)
  }

  @Test func validateReportsFormatErrorForNonBlankEmail() {
    let form = TFormState()
    let email = form.field(rules: [.required(Self.emailRequired), .email(Self.emailInvalid)])
    let password = form.field(rules: [.required(Self.passwordRequired)])
    email.value = "me@example"
    password.value = "secret"

    #expect(form.validate() == false)
    #expect(email.errors == [Self.emailInvalid])
    #expect(password.errors.isEmpty)
  }

  @Test func assigningValueClearsOnlyThatFieldErrors() {
    let form = TFormState()
    let email = form.field(rules: [.required(Self.emailRequired)])
    let password = form.field(rules: [.required(Self.passwordRequired)])

    #expect(form.validate() == false)
    email.value = "me@example.com"

    #expect(email.errors.isEmpty)
    #expect(email.error == nil)
    #expect(password.errors == [Self.passwordRequired])
  }

  @Test func errorsStayEmptyUntilValidate() {
    let form = TFormState()
    let email = form.field(rules: [.required(Self.emailRequired), .email(Self.emailInvalid)])
    let password = form.field(rules: [.required(Self.passwordRequired)])

    email.value = "not-an-email"
    email.value = ""
    password.value = ""

    #expect(email.errors.isEmpty)
    #expect(password.errors.isEmpty)
    #expect(email.focusRequest == 0)
  }

  @Test func focusChainFollowsRegistrationOrder() {
    let form = TFormState()
    let email = form.field(rules: [.required(Self.emailRequired)])
    let password = form.field(rules: [.required(Self.passwordRequired)])

    #expect(form.isFirst(email))
    #expect(form.isLast(email) == false)
    #expect(form.isFirst(password) == false)
    #expect(form.isLast(password))
  }

  @Test func focusNextMovesToFollowingField() {
    let form = TFormState()
    let email = form.field(rules: [.required(Self.emailRequired)])
    let password = form.field(rules: [.required(Self.passwordRequired)])

    form.focusNext(after: email)
    #expect(password.focusRequest == 1)
    #expect(email.focusRequest == 0)
  }

  @Test func focusNextOnLastFieldDoesNothing() {
    let form = TFormState()
    let email = form.field(rules: [.required(Self.emailRequired)])
    let password = form.field(rules: [.required(Self.passwordRequired)])

    form.focusNext(after: password)
    #expect(password.focusRequest == 0)
    #expect(email.focusRequest == 0)
  }

  @Test func repeatedValidateReplacesErrorsAndRequestsFocusAgain() {
    let form = TFormState()
    let email = form.field(rules: [.required(Self.emailRequired), .email(Self.emailInvalid)])
    #expect(!form.validate())
    #expect(!form.validate())
    #expect(email.errors == [Self.emailRequired])
    #expect(email.focusRequest == 2)
  }

  @Test func emailRuleNeitherTrimsNorLowercases() {
    let rule = TFormRule.email(Self.emailInvalid)
    #expect(rule.validate("ME@Example.COM") == nil)
    #expect(rule.validate(" a@b.co") == Self.emailInvalid)
    #expect(rule.validate("a@b.co ") == Self.emailInvalid)
  }

  @Test func fieldKeepsInitialValue() {
    let form = TFormState(autoFocusFirstField: false)
    let email = form.field("me@example.com", rules: [.required(Self.emailRequired)])

    #expect(email.value == "me@example.com")
    #expect(form.autoFocusFirstField == false)
    #expect(TFormState().autoFocusFirstField)
  }

  @Test func blankBlurDoesNotStartValidation() {
    let form = TFormState(validatesOnBlur: true)
    let email = form.field(rules: [.required(Self.emailRequired), .email(Self.emailInvalid)])

    email.isFocused = true
    email.editingEnded()
    #expect(email.isFocused == false)
    #expect(email.errors.isEmpty)
    #expect(email.isVerified == false)

    email.value = "me@"
    #expect(email.errors.isEmpty)
    #expect(email.isVerified == false)
  }

  @Test func firstEntryDefersValidationUntilBlur() {
    let form = TFormState(validatesOnBlur: true)
    let email = form.field(rules: [.required(Self.emailRequired), .email(Self.emailInvalid)])

    email.value = "me@example"
    #expect(email.errors.isEmpty)
    #expect(email.isVerified == false)

    email.editingEnded()
    #expect(email.errors == [Self.emailInvalid])
    #expect(email.isVerified == false)
  }

  @Test func validBlurVerifiesValue() {
    let form = TFormState(validatesOnBlur: true)
    let email = form.field(rules: [.required(Self.emailRequired), .email(Self.emailInvalid)])

    email.value = "me@example.com"
    email.editingEnded()
    #expect(email.errors.isEmpty)
    #expect(email.isVerified)
  }

  @Test func everyChangeRevalidatesOnceValidated() {
    let form = TFormState(validatesOnBlur: true)
    let email = form.field(rules: [.required(Self.emailRequired), .email(Self.emailInvalid)])
    email.value = "me@ex."
    email.editingEnded()
    #expect(email.errors == [Self.emailInvalid])

    email.value = "me@ex.c"
    #expect(email.errors == [Self.emailInvalid])
    #expect(email.isVerified == false)

    email.value = "me@ex.co"
    #expect(email.errors.isEmpty)
    #expect(email.isVerified)

    email.value = "me@ex.co!"
    #expect(email.errors == [Self.emailInvalid])
    #expect(email.isVerified == false)

    email.value = ""
    #expect(email.errors == [Self.emailRequired])
    #expect(email.isVerified == false)

    email.value = "me@ex.com"
    #expect(email.errors.isEmpty)
    #expect(email.isVerified)
  }

  @Test func submitStartsLiveValidationForEveryField() {
    let form = TFormState(validatesOnBlur: true)
    let email = form.field(rules: [.required(Self.emailRequired), .email(Self.emailInvalid)])
    let password = form.field(rules: [.required(Self.passwordRequired)])
    email.value = "me@example.com"
    email.editingEnded()
    #expect(email.isVerified)

    #expect(form.validate() == false)
    #expect(email.isVerified)
    #expect(password.errors == [Self.passwordRequired])

    password.value = "s"
    #expect(password.errors.isEmpty)
    #expect(password.isVerified)
    password.value = ""
    #expect(password.errors == [Self.passwordRequired])
    #expect(password.isVerified == false)
  }

  @Test func blankBlurKeepsErrorsSetBySubmit() {
    let form = TFormState(validatesOnBlur: true)
    let email = form.field(rules: [.required(Self.emailRequired), .email(Self.emailInvalid)])
    let password = form.field(rules: [.required(Self.passwordRequired)])
    password.isFocused = true

    #expect(form.validate() == false)
    password.editingEnded()

    #expect(password.errors == [Self.passwordRequired])
    #expect(email.errors == [Self.emailRequired])
    #expect(password.isFocused == false)
  }

  @Test func defaultFormSkipsBlurButRevalidatesAfterSubmit() {
    let form = TFormState()
    let email = form.field(rules: [.required(Self.emailRequired), .email(Self.emailInvalid)])
    email.value = "me@example"
    email.editingEnded()
    #expect(email.errors.isEmpty)
    #expect(email.isVerified == false)
    #expect(form.validatesOnBlur == false)

    #expect(form.validate() == false)
    email.value = "me@example.com"
    #expect(email.errors.isEmpty)
    email.value = "me@example"
    #expect(email.errors == [Self.emailInvalid])
  }

  @Test func endEditingResignsOnlyFocusedFields() {
    let form = TFormState()
    let email = form.field(rules: [.required(Self.emailRequired)])
    let password = form.field(rules: [.required(Self.passwordRequired)])
    password.isFocused = true

    form.endEditing()

    #expect(password.resignRequest == 1)
    #expect(email.resignRequest == 0)
  }
}
