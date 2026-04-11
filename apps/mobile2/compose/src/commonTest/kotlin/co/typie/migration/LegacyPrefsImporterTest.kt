package co.typie.migration

import co.typie.storage.Preference
import co.typie.ui.theme.ThemeMode
import kotlin.test.BeforeTest
import kotlin.test.Test
import kotlin.test.assertEquals

class LegacyPrefsImporterTest {
  @BeforeTest
  fun resetState() {
    Preference.legacySiteId = ""
    Preference.devMode = Preference.DEFAULT_DEV_MODE
    Preference.typewriterEnabled = Preference.DEFAULT_TYPEWRITER_ENABLED
    Preference.typewriterPosition = Preference.DEFAULT_TYPEWRITER_POSITION
    Preference.lineHighlightEnabled = Preference.DEFAULT_LINE_HIGHLIGHT_ENABLED
    Preference.autoSurroundEnabled = Preference.DEFAULT_AUTO_SURROUND_ENABLED
    Preference.characterCountFloatingEnabled = Preference.DEFAULT_CHARACTER_COUNT_FLOATING_ENABLED
    Preference.widgetAutoFadeEnabled = Preference.DEFAULT_WIDGET_AUTO_FADE_ENABLED
    Preference.themeMode = ThemeMode.System
    Preference.migrationSchemaVersion = 0
    Preference.migrationLastResultName = LegacyMigrationPhaseStatus.NotStarted.name
    Preference.migrationLastAttemptAtMillis = 0L
    Preference.migrationCompletedAtMillis = 0L
    Preference.migrationHandledSession = false
    Preference.migrationImportedSession = false
    Preference.migrationImportedPrefs = false
    Preference.migrationImportedPrefKeys = emptyList()
    Preference.migrationSkippedPrefKeys = emptyList()
  }

  @Test
  fun `import maps Flutter dark theme to KMP ThemeMode Dark`() {
    LegacyPrefsImporter.import(
      LegacyPrefsImportSource(preferenceValues = emptyMap(), themeValues = mapOf("mode" to "dark"))
    )
    assertEquals(ThemeMode.Dark, Preference.themeMode)
  }

  @Test
  fun `import only applies whitelisted keys`() {
    val report =
      LegacyPrefsImporter.import(
        LegacyPrefsImportSource(
          preferenceValues =
            mapOf(
              "site_id" to "site_fixture",
              "dev_mode" to true,
              "unexpected_key" to "ignored",
              "character_count_floating_position" to mapOf("x" to 0.1),
            ),
          themeValues = mapOf("mode" to "dark", "secondary" to "ignored"),
        )
      )
    assertEquals("site_fixture", Preference.legacySiteId)
    assertEquals(true, Preference.devMode)
    assertEquals(ThemeMode.Dark, Preference.themeMode)
    assertEquals(listOf("dev_mode", "site_id", "theme_mode"), report.importedKeys)
    assertEquals(emptyList(), report.skippedKeys)
  }

  @Test
  fun `import skips keys whose KMP target already has a non-default value`() {
    Preference.typewriterEnabled = true
    Preference.themeMode = ThemeMode.Light

    val report =
      LegacyPrefsImporter.import(
        LegacyPrefsImportSource(
          preferenceValues = mapOf(Preference.TYPEWRITER_ENABLED_KEY to false),
          themeValues = mapOf("mode" to "dark"),
        )
      )
    assertEquals(true, Preference.typewriterEnabled)
    assertEquals(ThemeMode.Light, Preference.themeMode)
    assertEquals(emptyList(), report.importedKeys)
    assertEquals(
      listOf("theme_mode", Preference.TYPEWRITER_ENABLED_KEY).sorted(),
      report.skippedKeys,
    )
  }

  @Test
  fun `import records partial-success state when some keys import and some are skipped`() {
    Preference.devMode = true

    val report =
      LegacyPrefsImporter.import(
        LegacyPrefsImportSource(
          preferenceValues = mapOf("site_id" to "site_fixture", Preference.DEV_MODE_KEY to false),
          themeValues = mapOf("mode" to "dark"),
        )
      )
    val state = LegacyMigrationStateStore.snapshot()

    assertEquals(listOf("site_id", "theme_mode"), report.importedKeys)
    assertEquals(listOf(Preference.DEV_MODE_KEY), report.skippedKeys)
    assertEquals(LegacyMigrationPhaseStatus.PartiallyImported, state.lastResult)
    assertEquals(listOf("site_id", "theme_mode"), state.importedPrefKeys)
    assertEquals(listOf(Preference.DEV_MODE_KEY), state.skippedPrefKeys)
    assertEquals(true, state.importedPrefs)
    assertEquals(true, Preference.devMode)
  }

  @Test
  fun `import does not reapply handled prefs after user resets them to defaults`() {
    val source =
      LegacyPrefsImportSource(
        preferenceValues = mapOf(Preference.DEV_MODE_KEY to true),
        themeValues = mapOf("mode" to "dark"),
      )

    LegacyPrefsImporter.import(source)
    Preference.devMode = Preference.DEFAULT_DEV_MODE
    Preference.themeMode = ThemeMode.System

    val report = LegacyPrefsImporter.import(source)

    assertEquals(Preference.DEFAULT_DEV_MODE, Preference.devMode)
    assertEquals(ThemeMode.System, Preference.themeMode)
    assertEquals(emptyList(), report.importedKeys)
    assertEquals(listOf(Preference.DEV_MODE_KEY, "theme_mode"), report.skippedKeys.sorted())
  }
}
