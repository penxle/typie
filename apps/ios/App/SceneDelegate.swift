import Design
import UIKit

final class SceneDelegate: UIResponder, UIWindowSceneDelegate {
  var window: UIWindow?
  private var toastWindow: ToastWindow?
  private var theme: ThemeSettings?

  func scene(
    _ scene: UIScene,
    willConnectTo session: UISceneSession,
    options connectionOptions: UIScene.ConnectionOptions
  ) {
    guard let windowScene = scene as? UIWindowScene,
      let environment = (UIApplication.shared.delegate as? AppDelegate)?.environment
    else { return }
    let window = UIWindow(windowScene: windowScene)
    let theme = ThemeSettings()
    self.theme = theme
    window.rootViewController = RootViewController(environment: environment, theme: theme)
    self.window = window
    toastWindow = ToastWindow(windowScene: windowScene, toast: environment.toast)
    observeTheme(theme, window: window)
    window.makeKeyAndVisible()
    handle(connectionOptions.urlContexts)
  }

  func scene(_ scene: UIScene, openURLContexts URLContexts: Set<UIOpenURLContext>) {
    handle(URLContexts)
  }

  private func handle(_ contexts: Set<UIOpenURLContext>) {
    for context in contexts {
      _ = SingleSignOnSDK.handle(context.url)
    }
  }

  func sceneDidDisconnect(_ scene: UIScene) {
    window = nil
    toastWindow = nil
    theme = nil
  }

  private func observeTheme(_ theme: ThemeSettings, window: UIWindow) {
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
    }
  }
}
