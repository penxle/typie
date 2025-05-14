package co.typie

import co.typie.webview.AppWebViewFactory
import io.flutter.embedding.android.FlutterActivity
import io.flutter.embedding.engine.FlutterEngine

class MainActivity : FlutterActivity() {
  override fun configureFlutterEngine(flutterEngine: FlutterEngine) {
    super.configureFlutterEngine(flutterEngine)

    flutterEngine.platformViewsController.registry.registerViewFactory(
      "co.typie.webview",
      AppWebViewFactory(flutterEngine.dartExecutor.binaryMessenger)
    )
  }
}
