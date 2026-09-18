import SwiftUI

public struct TShadow: Sendable {
  public let color: Color
  public let radius: CGFloat
  public let x: CGFloat
  public let y: CGFloat

  public init(color: Color, radius: CGFloat, x: CGFloat = 0, y: CGFloat) {
    self.color = color
    self.radius = radius
    self.x = x
    self.y = y
  }

  public static let empty = TShadow(color: .clear, radius: 0, y: 0)
}

public struct TShadows: Sendable {
  public let sm: TShadow
  public let md: TShadow
  public let lg: TShadow
  public let xl: TShadow

  static let lightBase = Color(argb: 0xFF18160F)

  public static let light = TShadows(
    sm: TShadow(color: lightBase.opacity(0.06), radius: 2, y: 1),
    md: TShadow(color: lightBase.opacity(0.08), radius: 4, y: 2),
    lg: TShadow(color: lightBase.opacity(0.10), radius: 8, y: 4),
    xl: TShadow(color: lightBase.opacity(0.12), radius: 16, y: 8))

  public static let dark = TShadows(sm: .empty, md: .empty, lg: .empty, xl: .empty)
}

extension ShapeStyle {
  public func shadow(_ shadow: TShadow) -> some ShapeStyle {
    self.shadow(.drop(color: shadow.color, radius: shadow.radius, x: shadow.x, y: shadow.y))
  }
}
