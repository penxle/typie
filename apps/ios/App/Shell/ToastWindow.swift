import Design
import UIKit

final class ToastWindow: UIWindow {
  init(windowScene: UIWindowScene, toast: TToastCenter) {
    super.init(windowScene: windowScene)
    windowLevel = .alert - 1
    isUserInteractionEnabled = false
    backgroundColor = .clear

    let root = UIViewController()
    root.view.backgroundColor = .clear

    let content = ThemedHostingController.make(title: "", TToastView(center: toast))
    content.view.backgroundColor = .clear
    content.view.translatesAutoresizingMaskIntoConstraints = false
    root.addChild(content)
    root.view.addSubview(content.view)
    NSLayoutConstraint.activate([
      content.view.leadingAnchor.constraint(equalTo: root.view.leadingAnchor),
      content.view.trailingAnchor.constraint(equalTo: root.view.trailingAnchor),
      content.view.topAnchor.constraint(equalTo: root.view.topAnchor),
      content.view.bottomAnchor.constraint(
        equalTo: root.view.keyboardLayoutGuide.topAnchor, constant: -12),
    ])
    content.didMove(toParent: root)

    rootViewController = root
    isHidden = false
  }

  @available(*, unavailable)
  required init?(coder: NSCoder) {
    nil
  }
}
