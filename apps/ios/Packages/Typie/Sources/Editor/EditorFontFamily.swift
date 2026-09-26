import Foundation

public struct EditorFontFamily: Equatable, Sendable {
  public enum Source: Equatable, Sendable {
    case `default`
    case user
    case fallback
  }

  public struct Font: Equatable, Sendable {
    public var weight: UInt16
    public var url: URL
    public var hash: String

    public init(weight: UInt16, url: URL, hash: String) {
      self.weight = weight
      self.url = url
      self.hash = hash
    }
  }

  public var name: String
  public var source: Source
  public var fonts: [Font]

  public init(name: String, source: Source, fonts: [Font]) {
    self.name = name
    self.source = source
    self.fonts = fonts
  }
}
