package co.typie.keyboard

import android.app.Activity
import android.view.WindowInsets
import io.flutter.plugin.common.BinaryMessenger
import io.flutter.plugin.common.EventChannel

class KeyboardPlugin(activity: Activity, messenger: BinaryMessenger) : EventChannel.StreamHandler {
  private val channel: EventChannel = EventChannel(messenger, "co.typie.keyboard")
  private var events: EventChannel.EventSink? = null

  init {
    channel.setStreamHandler(this)

    activity.window.decorView.setOnApplyWindowInsetsListener { view, originalInsets ->
      val insets = view.onApplyWindowInsets(originalInsets)

      val height =
        insets.getInsets(WindowInsets.Type.ime()).bottom / view.resources.displayMetrics.density
      events?.success(mapOf("height" to height))

      insets
    }
  }

  override fun onListen(arguments: Any?, events: EventChannel.EventSink?) {
    this.events = events
  }

  override fun onCancel(arguments: Any?) {
    events = null
  }
}