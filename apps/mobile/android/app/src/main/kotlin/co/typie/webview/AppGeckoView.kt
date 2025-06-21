package co.typie.webview

import android.annotation.SuppressLint
import android.content.Context
import android.os.Handler
import android.os.Looper
import android.view.View
import android.view.WindowInsets
import androidx.core.view.WindowInsetsCompat
import co.typie.keyboard.KeyboardPlugin
import io.flutter.plugin.common.BinaryMessenger
import io.flutter.plugin.common.MethodCall
import io.flutter.plugin.common.MethodChannel
import io.flutter.plugin.common.MethodChannel.MethodCallHandler
import io.flutter.plugin.platform.PlatformView
import org.json.JSONObject
import org.mozilla.geckoview.AllowOrDeny
import org.mozilla.geckoview.GeckoResult
import org.mozilla.geckoview.GeckoRuntime
import org.mozilla.geckoview.GeckoRuntimeSettings
import org.mozilla.geckoview.GeckoSession
import org.mozilla.geckoview.GeckoSession.ContentDelegate
import org.mozilla.geckoview.GeckoSession.NavigationDelegate
import org.mozilla.geckoview.GeckoView
import org.mozilla.geckoview.WebExtension.MessageDelegate
import org.mozilla.geckoview.WebExtension.Port
import org.mozilla.geckoview.WebExtension.PortDelegate

class AppGeckoView(
  context: Context, messenger: BinaryMessenger, id: Int, params: Map<*, *>?
) : PlatformView, MethodCallHandler {
  companion object {
    private var runtime: GeckoRuntime? = null
  }

  private val channel = MethodChannel(messenger, "co.typie.webview.gecko.$id")
  private val handler = Handler(Looper.getMainLooper())

  private val gecko = GeckoView(context)
  private val session = GeckoSession()

  private var extPort: Port? = null

  private var pendingCookies: List<Map<String, Any>>? = null
  private var pendingUrl: String? = null

  init {
    channel.setMethodCallHandler(this)

    gecko.setLayerType(View.LAYER_TYPE_HARDWARE, null)

    if (runtime == null) {
      runtime = GeckoRuntime.create(
        context, GeckoRuntimeSettings.Builder().fissionEnabled(true).build()
      )

      runtime!!.settings.apply {
        consoleOutputEnabled = true
        inputAutoZoomEnabled = false
        doubleTapZoomingEnabled = false
      }
    }

    gecko.apply {
      overScrollMode = View.OVER_SCROLL_NEVER
      isVerticalScrollBarEnabled = false
      isHorizontalScrollBarEnabled = false
      isNestedScrollingEnabled = true
      scrollBarStyle = View.SCROLLBARS_INSIDE_OVERLAY

      addWindowInsetsListener("keyboard") { _, insets ->
        val height =
          insets.getInsets(WindowInsetsCompat.Type.ime()).bottom / view.resources.displayMetrics.density

        KeyboardPlugin.getInstance()?.notifyKeyboardHeight(height.toDouble())

        insets
      }
    }

    setupDelegates()

    session.open(runtime!!)
    gecko.setSession(session)

    setupExtensions()

    params?.let {
      val userAgent = params["userAgent"] as? String
      userAgent?.let { session.settings.userAgentOverride = userAgent }

      pendingUrl = it["initialUrl"] as? String
      @Suppress("UNCHECKED_CAST") val initialCookies =
        it["initialCookies"] as? List<Map<String, Any>>

      if (initialCookies.isNullOrEmpty()) {
        pendingUrl?.let { url ->
          session.loadUri(url)
          pendingUrl = null
        }
      } else {
        pendingCookies = initialCookies
      }
    }
  }

  override fun getView(): View = gecko

  override fun onMethodCall(call: MethodCall, result: MethodChannel.Result) {
    when (call.method) {
      "requestFocus" -> {
        gecko.requestFocus()
        view.windowInsetsController?.show(WindowInsets.Type.ime())
        result.success(null)
      }

      "clearFocus" -> {
        view.windowInsetsController?.hide(WindowInsets.Type.ime())
        result.success(null)
      }

      "emitEvent" -> {
        val name = call.argument<String>("name")!!
        val data = call.argument<String>("data")!!

        val message = mapOf("type" to "emitEvent", "name" to name, "data" to data)
        extPort?.postMessage(JSONObject(message))

        result.success(null)
      }

      "dispose" -> {
        dispose()
        result.success(null)
      }

      else -> result.notImplemented()
    }
  }

  override fun dispose() {
    channel.setMethodCallHandler(null)

    extPort?.disconnect()
    extPort = null

    if (session.isOpen) {
      session.close()
    }
  }

  private fun setupDelegates() {
    session.contentDelegate = object : ContentDelegate {}

    session.navigationDelegate = object : NavigationDelegate {
      override fun onLoadRequest(
        session: GeckoSession, request: NavigationDelegate.LoadRequest
      ): GeckoResult<AllowOrDeny> {
        if (request.uri.startsWith("picker://")) {
          handler.post {
            channel.invokeMethod(
              "emitEvent", mapOf(
                "name" to "picker", "data" to mapOf("url" to request.uri)
              )
            )
          }

          return GeckoResult.deny()
        }

        return GeckoResult.allow()
      }
    }
  }

  @SuppressLint("WrongThread")
  private fun setupExtensions() {
    val path = "resource://android/assets/extension/"
    runtime!!.webExtensionController.ensureBuiltIn(path, "extension@typie.co").accept { extension ->
      if (extension == null) {
        return@accept
      }

      extension.setMessageDelegate(object : MessageDelegate {
        override fun onConnect(port: Port) {
          extPort = port

          port.setDelegate(object : PortDelegate {
            override fun onPortMessage(message: Any, port: Port) {
              val json = message as JSONObject
              val type = json.optString("type")

              when (type) {
                "emitEvent" -> {
                  val name = json.optString("name")
                  val data = json.optString("data")

                  channel.invokeMethod(
                    "emitEvent", mapOf(
                      "name" to name, "data" to data
                    )
                  )
                }

                "cookiesSet" -> {
                  session.loadUri(pendingUrl!!)
                  pendingUrl = null
                }
              }
            }
          })

          if (pendingUrl == null || pendingCookies == null) {
            return
          }

          val cookies = pendingCookies!!.map { cookie ->
            mapOf(
              "url" to pendingUrl,
              "name" to cookie["name"],
              "value" to cookie["value"],
              "domain" to cookie["domain"],
              "path" to "/",
              "secure" to true,
              "httpOnly" to true,
              "sameSite" to "lax"
            )
          }

          val message = mapOf("type" to "setCookies", "cookies" to cookies)
          port.postMessage(JSONObject(message))
        }
      }, "webview")

      session.reload()
    }
  }
}
