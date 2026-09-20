import Core
import FactoryKit
import GraphQL
import Observation

enum SingleSignOnOutcome: Sendable, Equatable {
  case succeeded
  case cancelled
  case failed
}

@Observable
@MainActor
final class SingleSignOnModel {
  private(set) var activeProvider: Core.SingleSignOnProvider?

  @ObservationIgnored private let singleSignOn = Container.shared.singleSignOn()
  @ObservationIgnored private let client = Container.shared.graphQLClient()
  @ObservationIgnored private let onSuccess: @MainActor () -> Void

  init(onSuccess: @escaping @MainActor () -> Void) {
    self.onSuccess = onSuccess
  }

  var isBusy: Bool { activeProvider != nil }

  func signIn(
    with provider: Core.SingleSignOnProvider,
    presenter: @escaping @MainActor () -> AnyObject?
  ) async -> SingleSignOnOutcome {
    guard activeProvider == nil else { return .cancelled }
    activeProvider = provider
    defer { activeProvider = nil }
    do {
      let credential = try await singleSignOn(provider, presenter).authenticate()
      _ = try await client.perform(
        SingleSignOnLogin_AuthorizeSingleSignOn_Mutation(
          input: AuthorizeSingleSignOnInput(
            params: JSON(credential.params),
            provider: .case(Self.wireProvider(credential.provider)))))
      onSuccess()
      return .succeeded
    } catch SingleSignOnError.cancelled {
      return .cancelled
    } catch is CancellationError {
      return .cancelled
    } catch {
      return .failed
    }
  }

  private static func wireProvider(_ provider: Core.SingleSignOnProvider)
    -> GraphQL.SingleSignOnProvider
  {
    switch provider {
    case .google: .google
    case .kakao: .kakao
    case .naver: .naver
    case .apple: .apple
    }
  }
}
