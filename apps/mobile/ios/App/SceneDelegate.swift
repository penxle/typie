import Compose
import GoogleSignIn
import KakaoSDKAuth
import NidThirdPartyLogin
import UIKit

final class SceneDelegate: UIResponder, UIWindowSceneDelegate {
  var window: UIWindow?

  func scene(
    _ scene: UIScene,
    willConnectTo session: UISceneSession,
    options connectionOptions: UIScene.ConnectionOptions
  ) {
    guard let windowScene = scene as? UIWindowScene else {
      return
    }

    let window = UIWindow(windowScene: windowScene)
    let shortcutRegistry = NativeShortcutRegistry()
    let statusBarAppearanceController = NavigationStatusBarAppearanceController()
    let composeController = MainViewControllerKt.MainViewController(
      shortcutRegistry: shortcutRegistry,
      statusBarAppearanceController: statusBarAppearanceController
    )
    composeController.view.backgroundColor = .systemBackground
    let controller = ShortcutHostingViewController(
      content: composeController,
      shortcutRegistry: shortcutRegistry,
      statusBarAppearanceController: statusBarAppearanceController
    )
    window.backgroundColor = .systemBackground
    window.rootViewController = controller
    self.window = window
    window.makeKeyAndVisible()

    if !connectionOptions.urlContexts.isEmpty {
      handleOpenURLContexts(connectionOptions.urlContexts)
    }
  }

  func scene(_ scene: UIScene, openURLContexts URLContexts: Set<UIOpenURLContext>) {
    handleOpenURLContexts(URLContexts)
  }

  func sceneDidDisconnect(_ scene: UIScene) {
    window = nil
  }

  private func handleOpenURLContexts(_ urlContexts: Set<UIOpenURLContext>) {
    for context in urlContexts {
      if GIDSignIn.sharedInstance.handle(context.url) {
        continue
      }

      if AuthController.handleOpenUrl(url: context.url) {
        continue
      }

      _ = NidOAuth.shared.handleURL(context.url)
    }
  }
}
