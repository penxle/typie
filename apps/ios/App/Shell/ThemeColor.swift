import Design
import SwiftUI
import UIKit

extension UIColor {
  static func theme(_ keyPath: KeyPath<TColors, Color>) -> UIColor {
    UIColor { traits in
      let colors = traits.userInterfaceStyle == .dark ? TColors.dark : TColors.light
      return UIColor(colors[keyPath: keyPath])
    }
  }
}
