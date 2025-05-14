package co.typie.webview

import android.annotation.SuppressLint
import android.content.Context
import android.view.View
import android.webkit.ConsoleMessage
import android.webkit.ConsoleMessage.MessageLevel
import android.webkit.CookieManager
import android.webkit.WebChromeClient
import android.webkit.WebSettings
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
  private val channel = MethodChannel(messenger, "co.typie.webview.$id")

  private val webView = WebView(context)
  private val cookieManager = CookieManager.getInstance()

  init {
    channel.setMethodCallHandler(this)

    webView.setLayerType(View.LAYER_TYPE_HARDWARE, null)

    webView.settings.apply {
      javaScriptEnabled = true
      domStorageEnabled = true

      loadWithOverviewMode = true
      useWideViewPort = true
      setSupportZoom(false)

      cacheMode = WebSettings.LOAD_NO_CACHE
    }


    webView.webViewClient = object : WebViewClient() {}

    webView.webChromeClient = object : WebChromeClient() {
      override fun onConsoleMessage(consoleMessage: ConsoleMessage): Boolean {
        val message = consoleMessage.message()
        val level = when (consoleMessage.messageLevel()) {
          MessageLevel.LOG -> "LOG"
          MessageLevel.DEBUG -> "DEBUG"
          MessageLevel.TIP -> "INFO"
          MessageLevel.WARNING -> "WARN"
          MessageLevel.ERROR -> "ERROR"
          null -> "LOG"
        }

        channel.invokeMethod(
          "onConsole", mapOf("level" to level, "message" to message)
        )

        return true
      }
    }

    params?.let {
      val userAgent = params["userAgent"] as? String
      userAgent?.let { webView.settings.userAgentString = userAgent }

      val cookies = params["initialCookies"] as? List<*>
      cookies?.let {
        for (cookie in cookies) {
          val props = cookie as? Map<*, *>
          props?.let {
            val name = props["name"] as? String
            val value = props["value"] as? String
            val domain = props["domain"] as? String

            if (name != null && value != null && domain != null) {
              cookieManager.setCookie(
                "https://$domain",
                "$name=$value; Domain=$domain; Path=/; Secure; SameSite=Lax"
              )
            }
          }
        }
      }

      val initialUrl = params["initialUrl"] as? String
      initialUrl?.let { webView.loadUrl(it) }
    }
  }

  override fun getView(): View = webView

  override fun onMethodCall(call: MethodCall, result: Result) {
    when (call.method) {
      "dispose" -> {}
      else -> result.notImplemented()
    }
  }

  override fun dispose() {
    webView.destroy()
    channel.setMethodCallHandler(null)
  }
}