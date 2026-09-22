#if canImport(UIKit)

  import Core
  import Design
  import FactoryKit
  import UIKit

  public final class MainWindow: UIWindow {
    private static let toastSpacing: CGFloat = 12

    private var overlay: UIView?
    private var toastHost: UIViewController?
    private let layout = TToastLayout()
    private let chrome = Container.shared.bottomChrome()
    private var keyboardHeight: CGFloat = 0

    public func installToast(center: TToastCenter) {
      let host = ThemedHostingController(title: "", TToastOverlay(center: center, layout: layout))
      host.safeAreaRegions = []
      toastHost = host
      let view = host.view!
      overlay = view
      view.isUserInteractionEnabled = false
      view.translatesAutoresizingMaskIntoConstraints = false
      addSubview(view)
      NSLayoutConstraint.activate([
        view.leadingAnchor.constraint(equalTo: leadingAnchor),
        view.trailingAnchor.constraint(equalTo: trailingAnchor),
        view.topAnchor.constraint(equalTo: topAnchor),
        view.bottomAnchor.constraint(equalTo: bottomAnchor),
      ])
      NotificationCenter.default.addObserver(
        self, selector: #selector(keyboardWillChange(_:)),
        name: UIResponder.keyboardWillChangeFrameNotification, object: nil)
      keepObserving(while: self) { [weak self] in
        guard let self else { return }
        _ = chrome.inset
        syncInset()
      }
    }

    public override func safeAreaInsetsDidChange() {
      super.safeAreaInsetsDidChange()
      syncInset()
    }

    public override func didAddSubview(_ subview: UIView) {
      super.didAddSubview(subview)
      guard let overlay, subview !== overlay else { return }
      bringSubviewToFront(overlay)
    }

    @objc private func keyboardWillChange(_ notification: Notification) {
      guard let frame = notification.userInfo?[UIResponder.keyboardFrameEndUserInfoKey] as? CGRect
      else { return }
      let local = convert(frame, from: nil)
      keyboardHeight = max(0, bounds.maxY - local.minY)
      syncInset()
    }

    private func syncInset() {
      let floor = keyboardHeight > 0 ? keyboardHeight : max(chrome.inset, safeAreaInsets.bottom)
      let inset = floor + Self.toastSpacing
      guard layout.bottomInset != inset else { return }
      layout.bottomInset = inset
    }
  }

#endif
