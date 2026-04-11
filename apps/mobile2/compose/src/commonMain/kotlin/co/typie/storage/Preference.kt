package co.typie.storage

import co.typie.platform.PlatformModule
import co.typie.preflight.Preflight
import co.typie.ui.theme.ThemeMode
import eu.anifantakis.lib.ksafe.KSafeWriteMode
import eu.anifantakis.lib.ksafe.invoke

internal inline fun <reified T> prefs(key: String, defaultValue: T): PersistentStateFlow<T> {
  val delegate = PlatformModule.ksafePrefs.invoke(defaultValue, key, mode = KSafeWriteMode.Plain)
  val holder =
    object {
      var v: T by delegate
    }
  val initial = holder.v
  return PersistentStateFlow(initial) { holder.v = it }
}

object Preference {
  val themeMode = prefs("theme_mode", ThemeMode.System)

  val siteId = prefs<String?>("site_id", null)

  val recentSearches = prefs("recent_searches", emptyList<String>())

  val typewriterEnabled = prefs("typewriter_enabled", false)
  val typewriterPosition = prefs("typewriter_position", 0.5)
  val lineHighlightEnabled = prefs("line_highlight_enabled", true)
  val autoSurroundEnabled = prefs("auto_surround_enabled", true)
  val characterCountFloatingEnabled = prefs("character_count_floating_enabled", false)
  val widgetAutoFadeEnabled = prefs("widget_auto_fade_enabled", true)

  val devMode = prefs("dev_mode", false)

  val preflightCache = prefs<Preflight?>("preflight_cache", null)
}
