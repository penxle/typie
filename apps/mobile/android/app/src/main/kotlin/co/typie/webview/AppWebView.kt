package co.typie.webview

import android.annotation.SuppressLint
import android.content.Context
import android.view.View
import android.webkit.ConsoleMessage
import android.webkit.ConsoleMessage.MessageLevel
import android.webkit.WebChromeClient
import android.webkit.WebView
import android.webkit.WebViewClient
import io.flutter.plugin.common.BinaryMessenger
import io.flutter.plugin.common.MethodCall
import io.flutter.plugin.common.MethodChannel
import io.flutter.plugin.common.MethodChannel.MethodCallHandler
import io.flutter.plugin.common.MethodChannel.Result
import io.flutter.plugin.platform.PlatformView

@SuppressLint("SetJavaScriptEnabled")
class AppWebView(
  context: Context, messenger: BinaryMessenger, id: Int, params: Map<*, *>?
) : PlatformView, MethodCallHandler {

  private val webView: WebView = WebView(context)
  private val channel: MethodChannel = MethodChannel(messenger, "co.typie.webview/$id");

  init {
    channel.setMethodCallHandler(this)

    webView.settings.apply {
      javaScriptEnabled = true
      domStorageEnabled = true
      loadWithOverviewMode = true
      useWideViewPort = true
    }

    webView.webViewClient = object : WebViewClient() {
      override fun onPageFinished(view: WebView?, url: String?) {
        super.onPageFinished(view, url)
        channel.invokeMethod("onPageFinished", url)
      }
    }

    webView.webChromeClient = object : WebChromeClient() {
      override fun onConsoleMessage(consoleMessage: ConsoleMessage): Boolean {
        val message = consoleMessage.message();
        val level = when (consoleMessage.messageLevel()) {
          MessageLevel.LOG -> "LOG"
          MessageLevel.DEBUG -> "DEBUG"
          MessageLevel.TIP -> "INFO"
          MessageLevel.WARNING -> "WARN"
          MessageLevel.ERROR -> "ERROR"
          null -> "LOG"
        }

        channel.invokeMethod(
          "console", mapOf("level" to level, "message" to message)
        )

        return true
      }
    }

    params?.let {
      val initialUrl = params["initialUrl"] as? String
      initialUrl?.let { webView.loadUrl(it) }
    }
  }

  override fun getView(): View = webView

  override fun onMethodCall(call: MethodCall, result: Result) {
    when (call.method) {
      else -> result.notImplemented()
    }
  }

  override fun dispose() {
    webView.destroy()
    channel.setMethodCallHandler(null)
  }
}