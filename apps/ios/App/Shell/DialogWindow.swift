import Core
import Design
import UIKit

final class DialogWindow: UIWindow {
  init(windowScene: UIWindowScene, dialog: TDialogCenter) {
    super.init(windowScene: windowScene)
    windowLevel = .alert - 2
    isUserInteractionEnabled = false
    backgroundColor = .clear

    let root = UIViewController()
    root.view.backgroundColor = .clear

    let content = ThemedHostingController.make(title: "", TDialogOverlay(center: dialog))
    content.view.backgroundColor = .clear
    content.view.translatesAutoresizingMaskIntoConstraints = false
    root.addChild(content)
    root.view.addSubview(content.view)
    NSLayoutConstraint.activate([
      content.view.leadingAnchor.constraint(equalTo: root.view.leadingAnchor),
      content.view.trailingAnchor.constraint(equalTo: root.view.trailingAnchor),
      content.view.topAnchor.constraint(equalTo: root.view.topAnchor),
      content.view.bottomAnchor.constraint(equalTo: root.view.bottomAnchor),
    ])
    content.didMove(toParent: root)

    rootViewController = root
    isHidden = false

    keepObserving(while: self) { [weak self] in
      let presenting = dialog.current != nil
      self?.isUserInteractionEnabled = presenting
      self?.rootViewController?.view.accessibilityViewIsModal = presenting
    }
  }

  @available(*, unavailable)
  required init?(coder: NSCoder) {
    nil
  }
}
