import Core
import Observation

public enum SingleSignOnOutcome: Sendable, Equatable {
  case succeeded
  case cancelled
  case failed
}

@Observable
@MainActor
public final class SingleSignOnModel {
  public private(set) var activeProvider: SingleSignOnProvider?

  @ObservationIgnored private let login: @Sendable (SingleSignOnProvider) async throws -> Void
  @ObservationIgnored private let onSuccess: @MainActor () -> Void

  public init(
    login: @escaping @Sendable (SingleSignOnProvider) async throws -> Void,
    onSuccess: @escaping @MainActor () -> Void
  ) {
    self.login = login
    self.onSuccess = onSuccess
  }

  public var isBusy: Bool { activeProvider != nil }

  public func signIn(with provider: SingleSignOnProvider) async -> SingleSignOnOutcome {
    guard activeProvider == nil else { return .cancelled }
    activeProvider = provider
    defer { activeProvider = nil }
    do {
      try await login(provider)
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
}
