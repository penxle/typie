import ApolloTestSupport
import FactoryKit
import FactoryTesting
import GraphQL
import GraphQLMocks
import Testing

@testable import Core
@testable import Features

@MainActor
@Suite(.container) struct EmailLoginModelTests {
  nonisolated private static let emailRequired = "이메일을 입력해주세요."
  nonisolated private static let emailInvalid = "올바른 이메일 형식을 입력해주세요."
  nonisolated private static let passwordRequired = "비밀번호를 입력해주세요."
  nonisolated private static let validEmail = "Me@Example.COM"
  nonisolated private static let validPassword = "  a b  "

  private final class SuccessCounter {
    private(set) var count = 0

    func record() {
      count += 1
    }
  }

  private struct Context {
    let model: EmailLoginModel
    let client: FakeGraphQLClient
    let success: SuccessCounter
  }

  private static func loggedIn() async -> EmailLogin_LoginWithEmail_Mutation.Data {
    await EmailLogin_LoginWithEmail_Mutation.Data.from(
      Mock<GraphQLMocks.Mutation>(loginWithEmail: true))
  }

  private func makeContext(
    _ result: Result<EmailLogin_LoginWithEmail_Mutation.Data, any Error>? = nil
  ) -> Context {
    let client = FakeGraphQLClient()
    if let result { client.stub(result, for: EmailLogin_LoginWithEmail_Mutation.self) }
    registerFake(client)
    let success = SuccessCounter()
    return Context(
      model: EmailLoginModel(onSuccess: { success.record() }), client: client, success: success)
  }

  private static func apiError(_ code: String) -> Result<
    EmailLogin_LoginWithEmail_Mutation.Data, any Error
  > {
    .failure(APIError(code: code, message: nil))
  }

  private func fill(_ model: EmailLoginModel) {
    model.email.value = Self.validEmail
    model.password.value = Self.validPassword
  }

  @Test func emptySubmissionReportsFieldErrorsWithoutPerforming() async {
    let context = makeContext()

    let failure = await context.model.submit()

    #expect(failure == nil)
    #expect(context.model.isSubmitting == false)
    #expect(context.client.performedMutations.isEmpty)
    #expect(context.model.email.errors == [Self.emailRequired])
    #expect(context.model.password.errors == [Self.passwordRequired])
    #expect(context.success.count == 0)
  }

  @Test func malformedEmailReportsFormatErrorWithoutPerforming() async {
    let context = makeContext()
    context.model.email.value = "invalid-email"
    context.model.password.value = Self.validPassword

    let failure = await context.model.submit()

    #expect(failure == nil)
    #expect(context.model.isSubmitting == false)
    #expect(context.client.performedMutations.isEmpty)
    #expect(context.model.email.errors == [Self.emailInvalid])
    #expect(context.model.password.errors.isEmpty)
    #expect(context.success.count == 0)
  }

  @Test func successSendsRawInputAndReportsSuccess() async throws {
    let context = makeContext(.success(await Self.loggedIn()))
    fill(context.model)

    let failure = await context.model.submit()

    #expect(failure == nil)
    #expect(context.model.isSubmitting == false)
    #expect(context.success.count == 1)
    #expect(context.model.email.errors.isEmpty)
    #expect(context.model.password.errors.isEmpty)
    let performed = context.client.performed(EmailLogin_LoginWithEmail_Mutation.self)
    #expect(performed.count == 1)
    let input = try #require(performed.first?.input)
    #expect(input.email == Self.validEmail)
    #expect(input.password == Self.validPassword)
  }

  @Test func invalidCredentialsReportsFailureAndKeepsInput() async {
    let context = makeContext(Self.apiError("invalid_credentials"))
    fill(context.model)

    let failure = await context.model.submit()

    #expect(failure == .invalidCredentials)
    #expect(context.model.isSubmitting == false)
    #expect(context.success.count == 0)
    #expect(context.model.email.value == Self.validEmail)
    #expect(context.model.password.value == Self.validPassword)
  }

  @Test func passwordNotSetReportsFailureAndKeepsInput() async {
    let context = makeContext(Self.apiError("password_not_set"))
    fill(context.model)

    let failure = await context.model.submit()

    #expect(failure == .passwordNotSet)
    #expect(context.model.isSubmitting == false)
    #expect(context.success.count == 0)
    #expect(context.model.email.value == Self.validEmail)
    #expect(context.model.password.value == Self.validPassword)
  }

  @Test func unclassifiedAPIErrorReportsUnknownFailureAndKeepsInput() async {
    let context = makeContext(Self.apiError("rate_limited"))
    fill(context.model)

    let failure = await context.model.submit()

    #expect(failure == .unknown)
    #expect(context.model.isSubmitting == false)
    #expect(context.success.count == 0)
    #expect(context.model.email.value == Self.validEmail)
    #expect(context.model.password.value == Self.validPassword)
  }

  @Test func transportErrorReportsUnknownFailure() async {
    let context = makeContext(.failure(HTTPError.status(500)))
    fill(context.model)

    #expect(await context.model.submit() == .unknown)
    #expect(context.success.count == 0)
  }

  @Test func cancellationReportsNoFailure() async {
    let context = makeContext(.failure(CancellationError()))
    fill(context.model)

    let failure = await context.model.submit()

    #expect(failure == nil)
    #expect(context.model.isSubmitting == false)
    #expect(context.success.count == 0)
  }

  @Test func repeatedFailureReportsFailureEachTime() async {
    let context = makeContext(Self.apiError("invalid_credentials"))
    fill(context.model)

    let first = await context.model.submit()
    let second = await context.model.submit()

    #expect(first == .invalidCredentials)
    #expect(second == .invalidCredentials)
    #expect(context.client.performed(EmailLogin_LoginWithEmail_Mutation.self).count == 2)
  }

  @Test func formValidatesOnBlurAndFocusesEmailOnRequest() {
    let context = makeContext()
    #expect(context.model.form.autoFocusFirstField == false)
    #expect(context.model.form.validatesOnBlur)

    context.model.focusEmail()
    #expect(context.model.email.focusRequest == 1)
    #expect(context.model.password.focusRequest == 0)
  }

  @Test func discardPasswordClearsOnlyThePassword() {
    let context = makeContext()
    fill(context.model)

    context.model.discardPassword()

    #expect(context.model.password.value.isEmpty)
    #expect(context.model.email.value == Self.validEmail)
  }

  @Test(.timeLimit(.minutes(1))) func secondSubmitIsIgnoredWhileFirstIsInFlight() async throws {
    let context = makeContext(.success(await Self.loggedIn()))
    let gate = context.client.hold()
    fill(context.model)

    let first = Task { await context.model.submit() }
    try await waitOnMain { context.client.performedMutations.count == 1 }

    #expect(context.model.isSubmitting)

    let ignored = await context.model.submit()

    #expect(ignored == nil)
    #expect(context.client.performed(EmailLogin_LoginWithEmail_Mutation.self).count == 1)

    await gate.open()
    #expect(await first.value == nil)
    #expect(context.client.performed(EmailLogin_LoginWithEmail_Mutation.self).count == 1)
    #expect(context.model.isSubmitting == false)
    #expect(context.success.count == 1)
  }
}
