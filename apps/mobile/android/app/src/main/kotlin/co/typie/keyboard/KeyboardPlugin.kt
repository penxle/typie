package co.typie.keyboard

import android.app.Activity
import android.content.ComponentCallbacks2
import android.content.res.Configuration
import android.view.WindowInsets
import androidx.core.view.ViewCompat
import androidx.core.view.WindowCompat
import androidx.core.view.WindowInsetsCompat
import io.flutter.plugin.common.BinaryMessenger
import io.flutter.plugin.common.EventChannel

class KeyboardPlugin(private val activity: Activity, messenger: BinaryMessenger) :
  EventChannel.StreamHandler {
  companion object {
    @Volatile
    private var instance: KeyboardPlugin? = null
    
    fun getInstance(): KeyboardPlugin? = instance
  }
  
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
    instance = this
    channel.setStreamHandler(this)

    WindowCompat.setDecorFitsSystemWindows(activity.window, false)
    ViewCompat.setOnApplyWindowInsetsListener(activity.window.decorView.rootView) { view, originalInsets ->
      val insets = ViewCompat.onApplyWindowInsets(view, originalInsets)

      val height =
        insets.getInsets(WindowInsetsCompat.Type.ime()).bottom / view.resources.displayMetrics.density
      notifyKeyboardHeight(height.toDouble())

      insets
    }
  }
  
  fun notifyKeyboardHeight(height: Double) {
    events?.success(mapOf("type" to "height", "height" to height))
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