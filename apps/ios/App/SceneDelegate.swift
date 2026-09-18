import Design
import UIKit

final class SceneDelegate: UIResponder, UIWindowSceneDelegate {
  var window: UIWindow?
  private var theme: ThemeSettings?
  private var router: Router?

  func scene(
    _ scene: UIScene,
    willConnectTo session: UISceneSession,
    options connectionOptions: UIScene.ConnectionOptions
  ) {
    guard let windowScene = scene as? UIWindowScene else { return }
    let window = UIWindow(windowScene: windowScene)
    let theme = ThemeSettings()
    self.theme = theme
    let router = Router(theme: theme)
    self.router = router
    window.rootViewController = MainTabBarController(
      rootProvider: { tab in router.root(for: tab) },
      createAction: { tab in router.createAction(for: tab) })
    self.window = window
    observeTheme(theme, window: window)
    window.makeKeyAndVisible()
  }

  func sceneDidDisconnect(_ scene: UIScene) {
    window = nil
    theme = nil
    router = nil
  }

  private func observeTheme(_ theme: ThemeSettings, window: UIWindow) {
    withObservationTracking {
      window.overrideUserInterfaceStyle =
        switch theme.mode {
        case .system: .unspecified
        case .light: .light
        case .dark: .dark
        }
    } onChange: { [weak self, weak window, weak theme] in
      Task { @MainActor in
        guard let self, let window, let theme else { return }
        self.observeTheme(theme, window: window)
      }
    }
  }
}
