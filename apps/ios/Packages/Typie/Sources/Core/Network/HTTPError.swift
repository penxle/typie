import Foundation

public enum HTTPError: Error, Equatable, Sendable {
  case status(Int)
  case network(String)
  case malformedResponse(String)
}

extension HTTPError: LocalizedError {
  public var errorDescription: String? {
    switch self {
    case .status(let code): "HTTP \(code)"
    case .network(let description): description
    case .malformedResponse(let description): description
    }
  }
}

public struct InvalidCredentialsError: Error, Equatable, Sendable {
  public init() {}
}
