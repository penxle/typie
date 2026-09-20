#if canImport(UIKit)

  import Design
  import UIKit

  final class ShellNavigationController: UINavigationController {
    init(root: UIViewController) {
      super.init(navigationBarClass: PassthroughNavigationBar.self, toolbarClass: nil)
      viewControllers = [root]
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

  final class PassthroughNavigationBar: UINavigationBar {
    override func hitTest(_ point: CGPoint, with event: UIEvent?) -> UIView? {
      guard let item = topItem, item.isEmpty else { return super.hitTest(point, with: event) }
      return nil
    }
  }

  extension UINavigationItem {
    fileprivate var isEmpty: Bool {
      (title ?? "").isEmpty && titleView == nil && (leftBarButtonItems ?? []).isEmpty
        && (rightBarButtonItems ?? []).isEmpty && hidesBackButton
    }
  }

#endif
