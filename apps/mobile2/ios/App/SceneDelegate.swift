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
    let controller = MainViewControllerKt.MainViewController()
    controller.view.backgroundColor = .systemBackground
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
