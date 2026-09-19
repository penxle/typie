import Foundation

public struct ImageSource: Equatable, Sendable {
  public let url: URL
  public let width: Int
  public let height: Int

  public init(url: URL, width: Int, height: Int) {
    self.url = url
    self.width = width
    self.height = height
  }

  public init?(_ fragment: TImage_image) {
    guard let url = URL(string: fragment.url) else { return nil }
    self.init(url: url, width: fragment.width, height: fragment.height)
  }
}
