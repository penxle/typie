import Design
import UIKit

@main
final class AppDelegate: UIResponder, UIApplicationDelegate {
  let environment = AppEnvironment()

  func application(
    _ application: UIApplication,
    didFinishLaunchingWithOptions launchOptions: [UIApplication.LaunchOptionsKey: Any]? = nil
  ) -> Bool {
    TFonts.registerAll()
    if let config = environment.config {
      SingleSignOnSDK.configure(config)
    }
    return true
  }
}
