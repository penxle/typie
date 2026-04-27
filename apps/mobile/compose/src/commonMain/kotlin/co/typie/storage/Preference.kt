package co.typie.storage

import androidx.compose.runtime.getValue
import androidx.compose.runtime.setValue
import co.typie.domain.preflight.Preflight
import co.typie.platform.PlatformModule
import co.typie.ui.theme.ThemeMode
import eu.anifantakis.lib.ksafe.KSafeWriteMode
import eu.anifantakis.lib.ksafe.invoke
import kotlin.uuid.ExperimentalUuidApi
import kotlin.uuid.Uuid

internal inline fun <reified T> prefs(key: String, defaultValue: T): PersistentState<T> {
  val delegate = PlatformModule.ksafePrefs.invoke(defaultValue, key, mode = KSafeWriteMode.Plain)
  val holder =
    object {
      var v: T by delegate
    }
  val initial = holder.v
  return PersistentState(initial) { holder.v = it }
}

object Preference {
  var themeMode by prefs("theme_mode", ThemeMode.System)

  var siteId by prefs<String?>("site_id", null)

  var recentSearches by prefs("recent_searches", emptyList<String>())

  var typewriterEnabled by prefs("typewriter_enabled", false)
  var typewriterPosition by prefs("typewriter_position", 0.5)
  var lineHighlightEnabled by prefs("line_highlight_enabled", true)
  var autoSurroundEnabled by prefs("auto_surround_enabled", true)
  var characterCountFloatingEnabled by prefs("character_count_floating_enabled", false)
  var widgetAutoFadeEnabled by prefs("widget_auto_fade_enabled", true)

  var devMode by prefs("dev_mode", false)

  var preflightCache by prefs<Preflight?>("preflight_cache", null)

  private var _deviceId: String? by prefs<String?>("device_id", null)

  @OptIn(ExperimentalUuidApi::class)
  val deviceId: String
    get() = _deviceId ?: Uuid.random().toHexString().also { _deviceId = it }
}
