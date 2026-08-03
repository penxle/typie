package co.typie.storage

import androidx.compose.runtime.getValue
import androidx.compose.runtime.setValue
import co.typie.domain.preflight.Preflight
import co.typie.domain.subscription.EntitlementCache
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

internal val userScopedRegistry = UserScopedRegistry()

object Preference {
  var themeMode by prefs("theme_mode", ThemeMode.System)

  var siteId by userPrefs<String?>("site_id", null)

  var recentSearches by userPrefs("recent_searches", emptyList<String>())

  var typewriterEnabled by userPrefs("typewriter_enabled", false)
  var typewriterPosition by userPrefs("typewriter_position", 0.5)
  var lineHighlightEnabled by userPrefs("line_highlight_enabled", true)
  var doubleTapToEditEnabled by userPrefs("double_tap_to_edit_enabled", true)
  var autoSurroundEnabled by userPrefs("auto_surround_enabled", true)
  var searchMatchWholeWord by userPrefs("search_match_whole_word", false)
  var characterCountFloatingEnabled by userPrefs("character_count_floating_enabled", false)
  var characterCountFloatingPositionX by userPrefs("character_count_floating_position_x", 0.05)
  var characterCountFloatingPositionY by userPrefs("character_count_floating_position_y", 0.05)
  var widgetAutoFadeEnabled by userPrefs("widget_auto_fade_enabled", true)

  var devMode by prefs("dev_mode", false)

  var legacyMigrationHandled by prefs("legacy_migration_handled", false)

  var planChangeNoticeShown by userPrefs("plan_change_notice_shown", false)

  var trialReminderLastShownDate by userPrefs<String?>("trial_reminder_last_shown_date", null)

  var preflightCache by prefs<Preflight?>("preflight_cache", null)

  var entitlementCache by prefs<EntitlementCache?>("entitlement_cache", null)

  private var _deviceId: String? by prefs<String?>("device_id", null)

  @OptIn(ExperimentalUuidApi::class)
  val deviceId: String
    get() = _deviceId ?: Uuid.random().toHexString().also { _deviceId = it }

  fun switchUser(userId: String?) {
    userScopedRegistry.switchUser(userId)
  }
}
