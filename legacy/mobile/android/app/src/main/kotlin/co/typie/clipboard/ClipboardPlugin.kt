package co.typie.clipboard

import android.content.ClipData
import android.content.ClipboardManager
import android.content.Context
import io.flutter.embedding.engine.plugins.FlutterPlugin
import io.flutter.plugin.common.MethodCall
import io.flutter.plugin.common.MethodChannel

class ClipboardPlugin : FlutterPlugin, MethodChannel.MethodCallHandler {
  private lateinit var channel: MethodChannel
  private lateinit var context: Context

  override fun onAttachedToEngine(binding: FlutterPlugin.FlutterPluginBinding) {
    channel = MethodChannel(binding.binaryMessenger, "co.typie.clipboard")
    channel.setMethodCallHandler(this)
    context = binding.applicationContext
  }

  override fun onDetachedFromEngine(binding: FlutterPlugin.FlutterPluginBinding) {
    channel.setMethodCallHandler(null)
  }

  override fun onMethodCall(call: MethodCall, result: MethodChannel.Result) {
    when (call.method) {
      "setData" -> handleSetData(call, result)
      "getData" -> handleGetData(result)
      else -> result.notImplemented()
    }
  }

  private fun handleSetData(call: MethodCall, result: MethodChannel.Result) {
    val text = call.argument<String>("text")
    val html = call.argument<String>("html")

    if (text == null || html == null) {
      result.error("INVALID_ARGS", "Missing text or html", null)
      return
    }

    val clipboard = context.getSystemService(Context.CLIPBOARD_SERVICE) as ClipboardManager
    val clip = ClipData.newHtmlText("typie", text, html)
    clipboard.setPrimaryClip(clip)

    result.success(null)
  }

  private fun handleGetData(result: MethodChannel.Result) {
    val clipboard = context.getSystemService(Context.CLIPBOARD_SERVICE) as ClipboardManager
    val data = mutableMapOf<String, String?>()

    val clip = clipboard.primaryClip
    if (clip != null && clip.itemCount > 0) {
      val item = clip.getItemAt(0)
      data["text"] = item.coerceToText(context)?.toString()
      data["html"] = item.htmlText
    }

    result.success(data)
  }
}
