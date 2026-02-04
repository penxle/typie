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

    let editorInputRegistrar = self.registrar(forPlugin: "co.typie.editor_input")
    let editorInputFactory = EditorInputFactory(messenger: editorInputRegistrar!.messenger())
    editorInputRegistrar?.register(editorInputFactory, withId: "co.typie.editor_input")

    KeyboardPlugin.register(with: self.registrar(forPlugin: "co.typie.keyboard")!)

    EditorTexturePlugin.register(with: self.registrar(forPlugin: "co.typie.editor_texture")!)

    ClipboardPlugin.register(with: self.registrar(forPlugin: "co.typie.clipboard")!)

    return super.application(application, didFinishLaunchingWithOptions: launchOptions)
  }
}
