#if canImport(UIKit)

  import Design
  import UIKit

  final class ShellNavigationController: UINavigationController {
    init(root: UIViewController) {
      super.init(rootViewController: root)
      navigationBar.tintColor = .theme(\.textDefault)
      let appearance = UINavigationBarAppearance()
      appearance.configureWithDefaultBackground()
      let chevron = UIImage(
        named: LucideIcon.chevronLeft.assetName, in: TDesignBundle.bundle, with: nil)
      appearance.setBackIndicatorImage(chevron, transitionMaskImage: chevron)
      navigationBar.standardAppearance = appearance
      navigationBar.scrollEdgeAppearance = appearance
    }

    @available(*, unavailable)
    required init?(coder: NSCoder) {
      nil
    }
  }

#endif
