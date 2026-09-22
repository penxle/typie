#if canImport(UIKit)

  import Design
  import SwiftUI
  import UIKit

  extension UIColor {
    static func theme(_ keyPath: KeyPath<TColors, Color>) -> UIColor {
      theme { $0[keyPath: keyPath] }
    }

    static func theme(_ resolve: @escaping (TColors) -> Color) -> UIColor {
      UIColor { traits in
        let colors = traits.userInterfaceStyle == .dark ? TColors.dark : TColors.light
        return UIColor(resolve(colors))
      }
    }
  }

#endif
