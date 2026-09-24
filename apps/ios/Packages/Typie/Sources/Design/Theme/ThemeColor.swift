#if canImport(UIKit)

  import SwiftUI
  import UIKit

  extension UIColor {
    public static func theme(_ keyPath: KeyPath<TColors, Color>) -> UIColor {
      theme { $0[keyPath: keyPath] }
    }

    public static func theme(_ resolve: @escaping (TColors) -> Color) -> UIColor {
      UIColor { traits in
        let colors = traits.userInterfaceStyle == .dark ? TColors.dark : TColors.light
        return UIColor(resolve(colors))
      }
    }
  }

#endif
