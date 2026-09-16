import SwiftUI

public enum TShapes {
  public static let sm: CGFloat = 6
  public static let md: CGFloat = 12
  public static let lg: CGFloat = 16
  public static let xl: CGFloat = 24
  public static let full: CGFloat = 999
  public static let capsule = Capsule()

  public static func rounded(_ radius: CGFloat) -> RoundedRectangle {
    RoundedRectangle(cornerRadius: radius, style: .circular)
  }

  public static func squircle(_ radius: CGFloat) -> RoundedRectangle {
    RoundedRectangle(cornerRadius: radius, style: .continuous)
  }
}
