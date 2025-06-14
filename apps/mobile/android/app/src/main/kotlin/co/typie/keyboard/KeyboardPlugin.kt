package co.typie.keyboard

import android.app.Activity
import android.content.ComponentCallbacks2
import android.content.res.Configuration
import android.view.WindowInsets
import io.flutter.plugin.common.BinaryMessenger
import io.flutter.plugin.common.EventChannel

class KeyboardPlugin(private val activity: Activity, messenger: BinaryMessenger) :
  EventChannel.StreamHandler {
  private val channel = EventChannel(messenger, "co.typie.keyboard")
  private var events: EventChannel.EventSink? = null

  private var hasHardwareKeyboard = false

  private val configurationChangeListener = object : ComponentCallbacks2 {
    override fun onConfigurationChanged(newConfig: Configuration) {
      val hardware = checkHardwareKeyboard()
      if (hardware != hasHardwareKeyboard) {
        hasHardwareKeyboard = hardware
        events?.success(mapOf("type" to "hardware", "hardware" to hardware))
      }
    }

    @Deprecated("Deprecated in Java")
    override fun onLowMemory() {
    }

    override fun onTrimMemory(level: Int) {
    }
  }

  init {
    channel.setStreamHandler(this)

    activity.window.decorView.setOnApplyWindowInsetsListener { view, originalInsets ->
      val insets = view.onApplyWindowInsets(originalInsets)

      val height =
        insets.getInsets(WindowInsets.Type.ime()).bottom / view.resources.displayMetrics.density
      events?.success(mapOf("type" to "height", "height" to height))

      insets
    }
  }

  override fun onListen(arguments: Any?, events: EventChannel.EventSink?) {
    this.events = events

    activity.registerComponentCallbacks(configurationChangeListener)

    hasHardwareKeyboard = checkHardwareKeyboard()
    events?.success(mapOf("type" to "hardware", "hardware" to hasHardwareKeyboard))
  }

  override fun onCancel(arguments: Any?) {
    activity.unregisterComponentCallbacks(configurationChangeListener)

    events = null
  }

  private fun checkHardwareKeyboard(): Boolean {
    val config = activity.resources.configuration
    return config.keyboard != Configuration.KEYBOARD_NOKEYS && config.hardKeyboardHidden != Configuration.HARDKEYBOARDHIDDEN_YES
  }
}