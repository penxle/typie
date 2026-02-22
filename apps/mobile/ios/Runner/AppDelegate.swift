import Flutter
import UIKit
import NaverThirdPartyLogin

private func registerCustomPlugins(with registry: FlutterPluginRegistry) {
  let registrar = registry.registrar(forPlugin: "co.typie.webview")
  let factory = AppWebViewFactory(messenger: registrar!.messenger())
  registrar?.register(factory, withId: "co.typie.webview")

  let editorInputRegistrar = registry.registrar(forPlugin: "co.typie.editor_input")
  let editorInputFactory = EditorInputFactory(messenger: editorInputRegistrar!.messenger())
  editorInputRegistrar?.register(editorInputFactory, withId: "co.typie.editor_input")

  KeyboardPlugin.register(with: registry.registrar(forPlugin: "co.typie.keyboard")!)
  EditorTexturePlugin.register(with: registry.registrar(forPlugin: "co.typie.editor_texture")!)
  ClipboardPlugin.register(with: registry.registrar(forPlugin: "co.typie.clipboard")!)
}

@main
@objc class AppDelegate: FlutterAppDelegate {
  override func application(
    _ application: UIApplication,
    didFinishLaunchingWithOptions launchOptions: [UIApplication.LaunchOptionsKey: Any]?
  ) -> Bool {
    GeneratedPluginRegistrant.register(with: self)

    UNUserNotificationCenter.current().delegate = self as UNUserNotificationCenterDelegate

    registerCustomPlugins(with: self)

    return super.application(application, didFinishLaunchingWithOptions: launchOptions)
  }

  @available(iOS 13.0, *)
  override func application(
    _ application: UIApplication,
    configurationForConnecting connectingSceneSession: UISceneSession,
    options: UIScene.ConnectionOptions
  ) -> UISceneConfiguration {
    let configuration = UISceneConfiguration(
      name: "Default Configuration",
      sessionRole: connectingSceneSession.role
    )
    configuration.delegateClass = SceneDelegate.self
    return configuration
  }
}

@available(iOS 13.0, *)
class SceneDelegate: UIResponder, UIWindowSceneDelegate {
  var window: UIWindow?
  private var flutterEngine: FlutterEngine?

  func scene(
    _ scene: UIScene,
    willConnectTo session: UISceneSession,
    options connectionOptions: UIScene.ConnectionOptions
  ) {
    guard let windowScene = scene as? UIWindowScene else { return }

    window = UIWindow(windowScene: windowScene)

    let engine = FlutterEngine(name: "engine_\(session.persistentIdentifier)")
    engine.run()
    flutterEngine = engine

    GeneratedPluginRegistrant.register(with: engine)
    registerCustomPlugins(with: engine)

    let controller = FlutterViewController(engine: engine, nibName: nil, bundle: nil)
    _ = controller.loadDefaultSplashScreenView()
    let splashBackgroundColor = controller.splashScreenView?.backgroundColor ?? .white
    controller.view.backgroundColor = splashBackgroundColor
    window?.backgroundColor = splashBackgroundColor
    window?.rootViewController = controller
    window?.makeKeyAndVisible()

    if !connectionOptions.urlContexts.isEmpty {
      handleNaverOpenURLContexts(scene, URLContexts: connectionOptions.urlContexts)
    }
  }

  func scene(_ scene: UIScene, openURLContexts URLContexts: Set<UIOpenURLContext>) {
    handleNaverOpenURLContexts(scene, URLContexts: URLContexts)
  }

  func sceneDidDisconnect(_ scene: UIScene) {
    flutterEngine?.destroyContext()
    flutterEngine = nil
    window = nil
  }

  private func handleNaverOpenURLContexts(
    _ scene: UIScene,
    URLContexts: Set<UIOpenURLContext>
  ) {
    guard let connection = NaverThirdPartyLoginConnection.getSharedInstance() else { return }
    connection.scene(scene, openURLContexts: URLContexts)

    for context in URLContexts {
      var options: [UIApplication.OpenURLOptionsKey: Any] = [:]

      if let sourceApplication = context.options.sourceApplication {
        options[.sourceApplication] = sourceApplication
      }

      if let annotation = context.options.annotation {
        options[.annotation] = annotation
      }

      options[.openInPlace] = context.options.openInPlace

      _ = connection.application(UIApplication.shared, open: context.url, options: options)
    }
  }
}
