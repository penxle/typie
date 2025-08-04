import airbridge_flutter_sdk
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

    KeyboardPlugin.register(with: self.registrar(forPlugin: "co.typie.keyboard")!)
    
    AirbridgeFlutter.initializeSDK(name: "typie", token: "cee38499c2ba42cc834503cd819573ac")

    return super.application(application, didFinishLaunchingWithOptions: launchOptions)
  }
}
