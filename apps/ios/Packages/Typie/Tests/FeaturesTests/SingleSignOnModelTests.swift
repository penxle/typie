import ApolloTestSupport
import Core
import FactoryKit
import FactoryTesting
import GraphQL
import GraphQLMocks
import Testing

@testable import Features

@MainActor
private final class AdapterStub {
  private(set) var providers: [Core.SingleSignOnProvider] = []
  private(set) var presented: [AnyObject?] = []
  var error: (any Error)?

  private var gated = false
  private var gate: CheckedContinuation<Void, Never>?
  private var entry: CheckedContinuation<Void, Never>?

  func hold() {
    gated = true
  }

  func waitForEntry() async {
    guard gate == nil else { return }
    await withCheckedContinuation { entry = $0 }
  }

  func release() {
    gated = false
    gate?.resume()
    gate = nil
  }

  func make(
    _ provider: Core.SingleSignOnProvider, _ presenter: @escaping @MainActor () -> AnyObject?
  ) -> any SingleSignOnAdapter {
    providers.append(provider)
    presented.append(presenter())
    return Adapter(provider: provider, owner: self)
  }

  fileprivate func authenticate(_ provider: Core.SingleSignOnProvider) async throws
    -> SingleSignOnCredential
  {
    if gated, providers.count == 1 {
      await withCheckedContinuation { continuation in
        gate = continuation
        entry?.resume()
        entry = nil
      }
    }
    if let error { throw error }
    return SingleSignOnCredential(provider: provider, params: ["access_token": "tok"])
  }

  private struct Adapter: SingleSignOnAdapter {
    let provider: Core.SingleSignOnProvider
    let owner: AdapterStub

    func authenticate() async throws -> SingleSignOnCredential {
      try await owner.authenticate(provider)
    }
  }
}

@MainActor
@Suite(.container) struct SingleSignOnModelTests {
  private final class SuccessCounter {
    private(set) var count = 0

    func record() {
      count += 1
    }
  }

  private final class Host {}

  private struct Context {
    let model: SingleSignOnModel
    let client: FakeGraphQLClient
    let adapter: AdapterStub
    let success: SuccessCounter
    let host: Host
  }

  private static func authorized() async -> SingleSignOnLogin_AuthorizeSingleSignOn_Mutation.Data {
    await SingleSignOnLogin_AuthorizeSingleSignOn_Mutation.Data.from(
      Mock<GraphQLMocks.Mutation>(authorizeSingleSignOn: "session-token"))
  }

  private func makeContext(
    _ result: Result<SingleSignOnLogin_AuthorizeSingleSignOn_Mutation.Data, any Error>
  ) -> Context {
    let client = FakeGraphQLClient()
    client.stub(result, for: SingleSignOnLogin_AuthorizeSingleSignOn_Mutation.self)
    registerFake(client)
    let adapter = AdapterStub()
    Container.shared.singleSignOn.register {
      { provider, presenter in adapter.make(provider, presenter) }
    }
    let success = SuccessCounter()
    return Context(
      model: SingleSignOnModel(onSuccess: { success.record() }), client: client, adapter: adapter,
      success: success, host: Host())
  }

  private func signIn(_ context: Context, _ provider: Core.SingleSignOnProvider) async
    -> SingleSignOnOutcome
  {
    await context.model.signIn(with: provider, presenter: { [host = context.host] in host })
  }

  private func authorizedInput(_ context: Context) throws -> AuthorizeSingleSignOnInput {
    let performed = context.client.performed(
      SingleSignOnLogin_AuthorizeSingleSignOn_Mutation.self)
    return try #require(performed.last?.input)
  }

  @Test func successAuthorizesCredentialAndReportsSuccess() async throws {
    let context = makeContext(.success(await Self.authorized()))

    let outcome = await signIn(context, .kakao)

    #expect(outcome == .succeeded)
    #expect(context.adapter.providers == [.kakao])
    #expect(context.success.count == 1)
    #expect(context.model.activeProvider == nil)
    #expect(context.model.isBusy == false)
    let input = try authorizedInput(context)
    #expect(input.provider.rawValue == "KAKAO")
    #expect(input.params.value as? [String: String] == ["access_token": "tok"])
    #expect(input.referralCode.unwrapped == nil)
  }

  @Test func mapsEveryProviderToItsWireName() async throws {
    for (provider, name) in [
      (Core.SingleSignOnProvider.google, "GOOGLE"), (.kakao, "KAKAO"), (.naver, "NAVER"),
      (.apple, "APPLE"),
    ] {
      let context = makeContext(.success(await Self.authorized()))
      #expect(await signIn(context, provider) == .succeeded)
      #expect(try authorizedInput(context).provider.rawValue == name)
    }
  }

  @Test func passesThePresenterToTheAdapterFactory() async {
    let context = makeContext(.success(await Self.authorized()))

    _ = await signIn(context, .google)

    #expect(context.adapter.presented.count == 1)
    #expect(context.adapter.presented[0] === context.host)
  }

  @Test func userCancellationReportsCancelledWithoutAuthorizing() async {
    let context = makeContext(.success(await Self.authorized()))
    context.adapter.error = SingleSignOnError.cancelled

    let outcome = await signIn(context, .google)

    #expect(outcome == .cancelled)
    #expect(context.adapter.providers == [.google])
    #expect(context.client.performedMutations.isEmpty)
    #expect(context.success.count == 0)
    #expect(context.model.activeProvider == nil)
  }

  @Test func taskCancellationReportsCancelled() async {
    let context = makeContext(.success(await Self.authorized()))
    context.adapter.error = CancellationError()

    #expect(await signIn(context, .apple) == .cancelled)
    #expect(context.success.count == 0)
  }

  @Test func missingCredentialReportsFailed() async {
    let context = makeContext(.success(await Self.authorized()))
    context.adapter.error = SingleSignOnError.missingCredential

    #expect(await signIn(context, .google) == .failed)
    #expect(context.success.count == 0)
  }

  @Test func authorizationFailureReportsFailed() async {
    let context = makeContext(.failure(APIError(code: "rate_limited", message: "too many")))

    #expect(await signIn(context, .naver) == .failed)
    #expect(context.success.count == 0)
    #expect(context.model.activeProvider == nil)
  }

  @Test(.timeLimit(.minutes(1))) func secondTapIsIgnoredWhileFirstIsInFlight() async {
    let context = makeContext(.success(await Self.authorized()))
    context.adapter.hold()

    let first = Task { await signIn(context, .kakao) }
    await context.adapter.waitForEntry()

    #expect(context.model.activeProvider == .kakao)
    #expect(context.model.isBusy)

    #expect(await signIn(context, .naver) == .cancelled)
    #expect(context.adapter.providers == [.kakao])

    context.adapter.release()

    #expect(await first.value == .succeeded)
    #expect(context.adapter.providers == [.kakao])
    #expect(context.success.count == 1)
    #expect(context.model.activeProvider == nil)
  }
}
