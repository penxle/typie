import Foundation

public struct AuthTokens: Codable, Sendable, Equatable {
  public let sessionToken: String
  public let accessToken: String
  public let userId: String

  public init(sessionToken: String, accessToken: String, userId: String) {
    self.sessionToken = sessionToken
    self.accessToken = accessToken
    self.userId = userId
  }
}
