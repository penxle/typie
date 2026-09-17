import UIKit

extension UIViewController {
  func embed(_ child: UIViewController) {
    addChild(child)
    child.view.frame = view.bounds
    child.view.autoresizingMask = [.flexibleWidth, .flexibleHeight]
    view.addSubview(child.view)
    child.didMove(toParent: self)
  }
}
