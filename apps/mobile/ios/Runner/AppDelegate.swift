import Flutter
import UIKit

@main
@objc class AppDelegate: FlutterAppDelegate {
  override func application(
    _ application: UIApplication,
    didFinishLaunchingWithOptions launchOptions: [UIApplication.LaunchOptionsKey: Any]?
  ) -> Bool {
    GeneratedPluginRegistrant.register(with: self)

    UNUserNotificationCenter.current().delegate = self as UNUserNotificationCenterDelegate

    let registrar = self.registrar(forPlugin: "co.typie.webview")
    let factory = AppWebViewFactory(messenger: registrar!.messenger())
    registrar?.register(factory, withId: "co.typie.webview")

    return super.application(application, didFinishLaunchingWithOptions: launchOptions)
  }
}
