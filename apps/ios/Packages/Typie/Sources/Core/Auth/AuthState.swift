import Observation

public enum AuthState: Sendable, Equatable {
  case unauthenticated
  case authenticated(AuthTokens)
}

protocol AuthStatePublishing: Sendable {
  func publish(_ state: AuthState) async
}

@MainActor @Observable public final class AuthStateStore {
  public private(set) var state: AuthState = .unauthenticated

  public init() {}
}

extension AuthStateStore: AuthStatePublishing {
  func publish(_ state: AuthState) async {
    self.state = state
  }
}
