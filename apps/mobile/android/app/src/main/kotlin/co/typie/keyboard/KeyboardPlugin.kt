package co.typie.keyboard

import android.app.Activity
import android.content.ComponentCallbacks2
import android.content.ComponentName
import android.content.res.Configuration
import android.provider.Settings
import androidx.core.view.ViewCompat
import androidx.core.view.WindowInsetsCompat
import io.flutter.plugin.common.BinaryMessenger
import io.flutter.plugin.common.EventChannel
import io.flutter.plugin.common.MethodChannel

class KeyboardPlugin(private val activity: Activity, messenger: BinaryMessenger) :
  EventChannel.StreamHandler {
  private val eventChannel = EventChannel(messenger, "co.typie.keyboard.event")
  private val methodChannel = MethodChannel(messenger, "co.typie.keyboard.method")
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
    eventChannel.setStreamHandler(this)

    methodChannel.setMethodCallHandler { call, result ->
      when (call.method) {
        "getCurrentKeyboard" -> result.success(getCurrentKeyboard())
        else -> result.notImplemented()
      }
    }

    ViewCompat.setOnApplyWindowInsetsListener(activity.window.decorView) { view, insets ->
      val imeInsets = insets.getInsets(WindowInsetsCompat.Type.ime())
      val height = imeInsets.bottom / view.resources.displayMetrics.density
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

  private fun getCurrentKeyboard(): Map<String, String?>? {
    val ime = Settings.Secure.getString(activity.contentResolver, Settings.Secure.DEFAULT_INPUT_METHOD)
      ?: return null
    val packageName = ComponentName.unflattenFromString(ime)?.packageName ?: return mapOf("id" to ime, "version" to null)
    val version = try {
      activity.packageManager.getPackageInfo(packageName, 0).versionName
    } catch (e: Exception) {
      android.util.Log.e("KeyboardPlugin", "getPackageInfo failed for $packageName", e)
      null
    }
    return mapOf("id" to ime, "version" to version)
  }
}