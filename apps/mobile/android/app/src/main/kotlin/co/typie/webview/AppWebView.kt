package co.typie.webview

import android.annotation.SuppressLint
import android.content.Context
import android.graphics.Bitmap
import android.os.Handler
import android.os.Looper
import android.view.View
import android.view.WindowInsets
import android.webkit.ConsoleMessage
import android.webkit.ConsoleMessage.MessageLevel
import android.webkit.CookieManager
import android.webkit.JavascriptInterface
import android.webkit.WebChromeClient
import android.webkit.WebResourceRequest
import android.webkit.WebResourceResponse
import android.webkit.WebSettings
import android.webkit.WebView
import android.webkit.WebViewClient
import com.squareup.moshi.Moshi
import com.squareup.moshi.adapter
import io.flutter.plugin.common.BinaryMessenger
import io.flutter.plugin.common.MethodCall
import io.flutter.plugin.common.MethodChannel
import io.flutter.plugin.common.MethodChannel.MethodCallHandler
import io.flutter.plugin.common.MethodChannel.Result
import io.flutter.plugin.platform.PlatformView
import java.io.FileInputStream

@OptIn(ExperimentalStdlibApi::class)
@SuppressLint("SetJavaScriptEnabled")
class AppWebView(
  context: Context, messenger: BinaryMessenger, id: Int, params: Map<*, *>?
) : PlatformView, MethodCallHandler {
  private val channel = MethodChannel(messenger, "co.typie.webview.$id")
  private val handler = Handler(Looper.getMainLooper())

  private val webView = WebView(context)
  private val cookieManager = CookieManager.getInstance()

  private val adapter = Moshi.Builder().build().adapter<Map<String, Any?>>()

  init {
    channel.setMethodCallHandler(this)

    webView.setLayerType(View.LAYER_TYPE_HARDWARE, null)

    webView.settings.apply {
      javaScriptEnabled = true
      domStorageEnabled = true

      loadWithOverviewMode = true
      useWideViewPort = true
      setSupportZoom(false)

      cacheMode = WebSettings.LOAD_DEFAULT
    }

    webView.apply {
      overScrollMode = View.OVER_SCROLL_NEVER
      isVerticalScrollBarEnabled = false
      isHorizontalScrollBarEnabled = false
      isNestedScrollingEnabled = true
      scrollBarStyle = View.SCROLLBARS_INSIDE_OVERLAY
    }

    webView.webViewClient = object : WebViewClient() {
      override fun onPageStarted(view: WebView?, url: String?, favicon: Bitmap?) {
        super.onPageStarted(view, url, favicon)
        setupEventChannel()
      }

      override fun shouldInterceptRequest(
        view: WebView?, request: WebResourceRequest?
      ): WebResourceResponse? {
        when (request?.url?.scheme) {
          "picker" -> {
            val data = FileInputStream(request.url.path)
            val mimeType = request.url.getQueryParameter("type") ?: "application/octet-stream"
            return WebResourceResponse(mimeType, null, data)
          }
        }

        return null
      }
    }

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
          "console", mapOf("level" to level, "message" to message)
        )

        return true
      }
    }

    webView.addJavascriptInterface(this, "webViewHandlers")

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
                "https://$domain", "$name=$value; Domain=$domain; Path=/; Secure; SameSite=Lax"
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
    val args = call.arguments as? Map<*, *>
    if (args == null) {
      result.success(null)
      return
    }

    when (call.method) {
      "requestFocus" -> {
        webView.requestFocus()
        view.windowInsetsController?.show(WindowInsets.Type.ime())
        result.success(null)
      }

      "clearFocus" -> {
        view.windowInsetsController?.hide(WindowInsets.Type.ime())
        // webView.clearFocus()
        result.success(null)
      }

      "emitEvent" -> {
        val name = args["name"] as? String
        val data = args["data"] as? String

        if (name != null && data != null) {
          val escapedData = data.replace("\\", "\\\\").replace("\"", "\\\"").replace("\n", "\\n")
            .replace("\r", "\\r").replace("\t", "\\t")

          webView.evaluateJavascript(
            """
              window.dispatchEvent(new CustomEvent('__webview__', { 
                detail: { 
                  name: "$name", 
                  data: JSON.parse("$escapedData") 
                } 
              }));
            """,
            null,
          )
        }

        result.success(null)
      }

      "dispose" -> {
        result.success(null)
      }

      else -> result.notImplemented()
    }
  }

  override fun dispose() {
    webView.destroy()
    channel.setMethodCallHandler(null)
  }

  private fun setupEventChannel() {
    webView.evaluateJavascript(
      """
        (() => {
          const handlers = new WeakMap();
          window.__webview__ = {
            emitEvent: (name, data) => window.webViewHandlers.postMessage(JSON.stringify({
              name: 'emitEvent',
              attrs: { name, data: JSON.stringify(data ?? null) },
            })),
            addEventListener: (name, fn) => {
              const handler = (event) => { if (event.detail.name === name) fn(event.detail.data) };
              handlers.set(fn, handler);
              window.addEventListener('__webview__', handler);
            },
            removeEventListener: (name, fn) => {
              const handler = handlers.get(fn);
              if (handler) {
                window.removeEventListener('__webview__', handler);
              }
            },
          };
        })();
      """,
      null,
    )
  }

  @JavascriptInterface
  fun postMessage(message: String) {
    val body = adapter.fromJson(message) ?: return
    val name = body["name"] as? String ?: return
    val attrs = body["attrs"] as? Map<*, *> ?: return

    when (name) {
      "emitEvent" -> {
        handler.post {
          channel.invokeMethod(
            "emitEvent", mapOf(
              "name" to attrs["name"],
              "data" to attrs["data"],
            )
          )
        }
      }
    }
  }
}
