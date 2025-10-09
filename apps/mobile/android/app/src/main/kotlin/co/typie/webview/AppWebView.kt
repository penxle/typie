package co.typie.webview

import android.annotation.SuppressLint
import android.app.Activity
import android.content.Context
import android.graphics.Bitmap
import android.os.Handler
import android.os.Looper
import android.view.View
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
import androidx.core.view.WindowInsetsCompat
import androidx.core.view.WindowInsetsControllerCompat
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
  private val activity = context as Activity

  private val webView = WebView(context)
  private val cookieManager = CookieManager.getInstance()

  private val adapter = Moshi.Builder().build().adapter<Map<String, Any?>>()

  private val pendingCallProcedureResults = mutableMapOf<String, Result>()

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

      mediaPlaybackRequiresUserGesture = false
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
        WindowInsetsControllerCompat(activity.window, view).show(WindowInsetsCompat.Type.ime())
        result.success(null)
      }

      "clearFocus" -> {
        WindowInsetsControllerCompat(activity.window, view).hide(WindowInsetsCompat.Type.ime())
        // webView.clearFocus()
        result.success(null)
      }

      "emitEvent" -> {
        val name = args["name"] as? String
        val data = args["data"] as? String

        if (name != null && data != null) {
          webView.evaluateJavascript(
            """
              window.dispatchEvent(new CustomEvent('__webview__', { 
                detail: { 
                  name: "$name", 
                  data: JSON.parse("$data") 
                } 
              }));
            """,
            null,
          )
        }

        result.success(null)
      }

      "callProcedure" -> {
        val name = args["name"] as? String
        val data = args["data"] as? String

        if (name != null && data != null) {
          val callId = java.util.UUID.randomUUID().toString()
          pendingCallProcedureResults[callId] = result
          
          webView.evaluateJavascript(
            """
              (async () => {
                const callId = "$callId";
                try {
                  const result = await window.__webview__.callProcedure(
                  "$name",
                  JSON.parse("$data")
                  );
                  window.webViewHandlers.postMessage(JSON.stringify({
                    name: 'callProcedureResult',
                    attrs: { 
                      callId: callId,
                      success: true,
                      result: result
                    }
                  }));
                } catch (error) {
                  window.webViewHandlers.postMessage(JSON.stringify({
                    name: 'callProcedureResult',
                    attrs: { 
                      callId: callId,
                      success: false,
                      message: error.message || error.toString(),
                      stack: error.stack || ''
                    }
                  }));
                }
              })();
            """, null
          )
        } else {
          result.error("INVALID_ARGUMENTS", "Name and data are required", null)
        }
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
          const procedures = {};
          window.__webview__ = {
            platform: 'android',
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
            setProcedure: (name, fn) => {
              procedures[name] = fn;
            },
            callProcedure: async (name, data) => {
              const fn = procedures[name];
              if (!fn) {
                throw new Error('Procedure not found: ' + name);
              }
              const result = await fn(data);
              return JSON.stringify(result ?? null);
            }
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

      "callProcedureResult" -> {
        handler.post {
          val callId = attrs["callId"] as? String ?: return@post
          pendingCallProcedureResults[callId]?.let { result ->
            val success = attrs["success"] as? Boolean ?: false
            if (success) {
              result.success(attrs["result"] as? String)
            } else {
              val errorMessage = attrs["message"] as? String ?: "Unknown error"
              val stack = attrs["stack"] as? String
              result.error(
                "JS_EXECUTION_ERROR",
                errorMessage,
                stack
              )
            }
            pendingCallProcedureResults.remove(callId)
          }
        }
      }
    }
  }
}
