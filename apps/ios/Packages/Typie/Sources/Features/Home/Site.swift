import Foundation

public struct Site: Identifiable, Equatable, Sendable {
  public let id: String
  public let name: String
  public let url: String
  public let logo: URL?

  public init(id: String, name: String, url: String, logo: URL?) {
    self.id = id
    self.name = name
    self.url = url
    self.logo = logo
  }
}
