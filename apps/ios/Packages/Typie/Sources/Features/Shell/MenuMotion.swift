#if canImport(UIKit)

  import SwiftUI

  enum MenuMotion {
    private static let entranceDelay: TimeInterval = 0.06
    private static let entranceDuration: TimeInterval = 0.25
    private static let exitDuration: TimeInterval = 0.1
    private static let staggerBase: TimeInterval = 0.018
    private static let staggerSpan: TimeInterval = 0.14
    private static let staggerExponent = 1.4

    static func visibility(presented: Bool, stagger: TimeInterval, reduceMotion: Bool) -> Animation
    {
      guard presented else { return .easeOut(duration: exitDuration) }
      return .timingCurve(0.23, 1, 0.32, 1, duration: entranceDuration)
        .delay(entranceDelay + (reduceMotion ? 0 : stagger))
    }

    static func stagger(index: Int, count: Int) -> TimeInterval {
      let steps = max(count - 1, 1)
      let progress = Double(index) / Double(steps)
      return staggerBase * Double(index) + staggerSpan * pow(progress, staggerExponent)
    }
  }

  enum MenuStyle {
    static func highlight(_ scheme: ColorScheme) -> Color {
      scheme == .dark ? Color.white.opacity(0.16) : Color.black.opacity(0.08)
    }
  }

#endif
