import UIKit

extension UIApplication {
  @MainActor
  var activeWindow: UIWindow? {
    let windowScene =
      connectedScenes
      .compactMap { $0 as? UIWindowScene }
      .filter { $0.activationState == .foregroundActive }
      .first

    return windowScene?.windows.first { $0.isKeyWindow }
      ?? windowScene?.windows.first
  }

  @MainActor
  var presentingViewController: UIViewController? {
    var viewController = activeWindow?.rootViewController
    while let presented = viewController?.presentedViewController {
      viewController = presented
    }
    return viewController
  }
}
