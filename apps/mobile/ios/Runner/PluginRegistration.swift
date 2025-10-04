import Flutter

class PluginRegistration {
  static func registerCustomPlugins(with registry: FlutterPluginRegistry) {
    if let registrar = registry.registrar(forPlugin: "co.typie.webview") {
      let factory = AppWebViewFactory(messenger: registrar.messenger())
      registrar.register(factory, withId: "co.typie.webview")
    }

    if let keyboardRegistrar = registry.registrar(forPlugin: "co.typie.keyboard") {
      KeyboardPlugin.register(with: keyboardRegistrar)
    }
  }
}
