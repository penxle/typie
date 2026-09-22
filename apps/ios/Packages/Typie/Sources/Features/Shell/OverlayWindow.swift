#if canImport(UIKit)

  import Core
  import Design
  import SwiftUI
  import UIKit

  public final class OverlayWindow: UIWindow {
    public init(
      windowScene: UIWindowScene, level: UIWindow.Level, avoidsKeyboard: Bool,
      isInteractive: @escaping @MainActor () -> Bool, content: some View
    ) {
      super.init(windowScene: windowScene)
      windowLevel = level
      isUserInteractionEnabled = false
      backgroundColor = .clear

      let root = UIViewController()
      root.view.backgroundColor = .clear
      let host = ThemedHostingController(title: "", content)
      host.view.translatesAutoresizingMaskIntoConstraints = false
      root.addChild(host)
      root.view.addSubview(host.view)
      let bottom =
        avoidsKeyboard
        ? host.view.bottomAnchor.constraint(
          equalTo: root.view.keyboardLayoutGuide.topAnchor, constant: -12)
        : host.view.bottomAnchor.constraint(equalTo: root.view.bottomAnchor)
      NSLayoutConstraint.activate([
        host.view.leadingAnchor.constraint(equalTo: root.view.leadingAnchor),
        host.view.trailingAnchor.constraint(equalTo: root.view.trailingAnchor),
        host.view.topAnchor.constraint(equalTo: root.view.topAnchor),
        bottom,
      ])
      host.didMove(toParent: root)
      rootViewController = root
      isHidden = false

      keepObserving(while: self) { [weak self] in
        let interactive = isInteractive()
        guard let self else { return }
        isUserInteractionEnabled = interactive
        accessibilityViewIsModal = interactive
        rootViewController?.view.accessibilityViewIsModal = interactive
      }
    }

    @available(*, unavailable)
    public required init?(coder: NSCoder) {
      nil
    }
  }

#endif
