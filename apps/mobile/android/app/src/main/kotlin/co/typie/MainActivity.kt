package co.typie

import android.content.Intent
import android.os.Bundle
import androidx.core.splashscreen.SplashScreen.Companion.installSplashScreen
import co.typie.keyboard.KeyboardPlugin
import co.typie.webview.AppWebViewFactory
import io.flutter.embedding.android.FlutterActivity
import io.flutter.embedding.engine.FlutterEngine

class MainActivity : FlutterActivity() {
  private var isHandlingActivityResult = false

  override fun onCreate(savedInstanceState: Bundle?) {
    installSplashScreen()
    super.onCreate(savedInstanceState)
  }

  override fun configureFlutterEngine(flutterEngine: FlutterEngine) {
    super.configureFlutterEngine(flutterEngine)

    flutterEngine.platformViewsController.registry.registerViewFactory(
      "co.typie.webview",
      AppWebViewFactory(flutterEngine.dartExecutor.binaryMessenger)
    )

    KeyboardPlugin(this, flutterEngine.dartExecutor.binaryMessenger)
  }

  override fun onActivityResult(requestCode: Int, resultCode: Int, data: Intent?) {
    if (isHandlingActivityResult) {
      return
    }
    
    isHandlingActivityResult = true
    try {
      super.onActivityResult(requestCode, resultCode, data)
    } finally {
      isHandlingActivityResult = false
    }
  }
}
