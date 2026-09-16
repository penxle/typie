import Design
import UIKit

enum ShellNavigationController {
  @MainActor
  static func make(root: UIViewController) -> UINavigationController {
    let navigation = UINavigationController(rootViewController: root)
    navigation.navigationBar.tintColor = .theme(\.textDefault)
    let appearance = UINavigationBarAppearance()
    appearance.configureWithDefaultBackground()
    let chevron = UIImage(
      named: LucideIcon.chevronLeft.assetName, in: DesignBundle.bundle, with: nil)
    appearance.setBackIndicatorImage(chevron, transitionMaskImage: chevron)
    navigation.navigationBar.standardAppearance = appearance
    navigation.navigationBar.scrollEdgeAppearance = appearance
    return navigation
  }
}
