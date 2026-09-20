import Core
import Design
import FactoryKit
import Features
import Platform
import UIKit

final class SceneDelegate: UIResponder, UIWindowSceneDelegate {
  var window: UIWindow?
  private var toastWindow: OverlayWindow?
  private var dialogWindow: OverlayWindow?

  func scene(
    _ scene: UIScene,
    willConnectTo session: UISceneSession,
    options connectionOptions: UIScene.ConnectionOptions
  ) {
    guard let windowScene = scene as? UIWindowScene else { return }
    let window = UIWindow(windowScene: windowScene)
    window.rootViewController = RootViewController()
    self.window = window
    let dialog = Container.shared.dialog()
    toastWindow = OverlayWindow(
      windowScene: windowScene, level: .alert - 1, avoidsKeyboard: true, isInteractive: { false },
      content: TToastOverlay(center: Container.shared.toast()))
    dialogWindow = OverlayWindow(
      windowScene: windowScene, level: .alert - 2, avoidsKeyboard: false,
      isInteractive: { dialog.current != nil }, content: TDialogOverlay(center: dialog))
    observeTheme(Container.shared.theme(), window: window)
    window.makeKeyAndVisible()
    handle(connectionOptions.urlContexts)
  }

  func scene(_ scene: UIScene, openURLContexts URLContexts: Set<UIOpenURLContext>) {
    handle(URLContexts)
  }

  private func handle(_ contexts: Set<UIOpenURLContext>) {
    for context in contexts {
      _ = Platform.handle(context.url)
    }
  }

  func sceneDidDisconnect(_ scene: UIScene) {
    window = nil
    toastWindow = nil
    dialogWindow = nil
  }

  private func observeTheme(_ theme: TThemeSettings, window: UIWindow) {
    keepObserving(while: window) { [weak self, weak window, weak theme] in
      guard let theme else { return }
      let style: UIUserInterfaceStyle =
        switch theme.mode {
        case .system: .unspecified
        case .light: .light
        case .dark: .dark
        }
      window?.overrideUserInterfaceStyle = style
      self?.toastWindow?.overrideUserInterfaceStyle = style
      self?.dialogWindow?.overrideUserInterfaceStyle = style
    }
  }
}
