import Apollo
import Foundation

public struct TypieError: Error, Equatable, Sendable {
  public let code: String
  public let message: String?

  public init(code: String, message: String?) {
    self.code = code
    self.message = message
  }
}

public struct SessionEstablishmentError: Error, Sendable {
  public let underlying: any Error

  public init(underlying: any Error) {
    self.underlying = underlying
  }
}

extension TypieError: LocalizedError {
  public var errorDescription: String? { message ?? code }
}

extension SessionEstablishmentError: LocalizedError {
  public var errorDescription: String? { underlying.localizedDescription }
}

func mappedGraphQLError(_ error: GraphQLError) -> any Error {
  guard let extensions = error.extensions,
    extensions["type"] as? String == "TypieError",
    let code = extensions["code"] as? String
  else {
    return error
  }
  return TypieError(code: code, message: (extensions["message"] as? String) ?? error.message)
}
