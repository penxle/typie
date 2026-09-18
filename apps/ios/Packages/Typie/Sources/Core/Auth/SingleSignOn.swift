public enum SingleSignOnProvider: String, CaseIterable, Sendable {
  case google
  case kakao
  case naver
  case apple
}

public struct SingleSignOnCredential: Equatable, Sendable {
  public let provider: SingleSignOnProvider
  public let params: [String: String]

  public init(provider: SingleSignOnProvider, params: [String: String]) {
    self.provider = provider
    self.params = params
  }
}

public protocol SingleSignOnAdapter: Sendable {
  func authenticate() async throws -> SingleSignOnCredential
}

public enum SingleSignOnError: Error, Equatable, Sendable {
  case cancelled
  case missingCredential
  case noPresenter
}
