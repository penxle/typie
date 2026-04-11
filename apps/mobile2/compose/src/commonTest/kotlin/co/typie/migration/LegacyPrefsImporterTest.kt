package co.typie.migration

import co.typie.storage.Preference
import co.typie.ui.theme.ThemeMode
import kotlin.test.BeforeTest
import kotlin.test.Test
import kotlin.test.assertEquals

class LegacyPrefsImporterTest {
  @BeforeTest
  fun resetState() {
    Preference.legacySiteId.value = ""
    Preference.devMode.value = Preference.DEFAULT_DEV_MODE
    Preference.typewriterEnabled.value = Preference.DEFAULT_TYPEWRITER_ENABLED
    Preference.typewriterPosition.value = Preference.DEFAULT_TYPEWRITER_POSITION
    Preference.lineHighlightEnabled.value = Preference.DEFAULT_LINE_HIGHLIGHT_ENABLED
    Preference.autoSurroundEnabled.value = Preference.DEFAULT_AUTO_SURROUND_ENABLED
    Preference.characterCountFloatingEnabled.value =
      Preference.DEFAULT_CHARACTER_COUNT_FLOATING_ENABLED
    Preference.widgetAutoFadeEnabled.value = Preference.DEFAULT_WIDGET_AUTO_FADE_ENABLED
    Preference.themeMode.value = ThemeMode.System
    Preference.migrationSchemaVersion.value = 0
    Preference.migrationLastResultName.value = LegacyMigrationPhaseStatus.NotStarted.name
    Preference.migrationLastAttemptAtMillis.value = 0L
    Preference.migrationCompletedAtMillis.value = 0L
    Preference.migrationHandledSession.value = false
    Preference.migrationImportedSession.value = false
    Preference.migrationImportedPrefs.value = false
    Preference.migrationImportedPrefKeys.value = emptyList()
    Preference.migrationSkippedPrefKeys.value = emptyList()
  }

  @Test
  fun `import maps Flutter dark theme to KMP ThemeMode Dark`() {
    LegacyPrefsImporter.import(
      LegacyPrefsImportSource(preferenceValues = emptyMap(), themeValues = mapOf("mode" to "dark"))
    )
    assertEquals(ThemeMode.Dark, Preference.themeMode.value)
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
    assertEquals("site_fixture", Preference.legacySiteId.value)
    assertEquals(true, Preference.devMode.value)
    assertEquals(ThemeMode.Dark, Preference.themeMode.value)
    assertEquals(listOf("dev_mode", "site_id", "theme_mode"), report.importedKeys)
    assertEquals(emptyList(), report.skippedKeys)
  }

  @Test
  fun `import skips keys whose KMP target already has a non-default value`() {
    Preference.typewriterEnabled.value = true
    Preference.themeMode.value = ThemeMode.Light

    val report =
      LegacyPrefsImporter.import(
        LegacyPrefsImportSource(
          preferenceValues = mapOf(Preference.TYPEWRITER_ENABLED_KEY to false),
          themeValues = mapOf("mode" to "dark"),
        )
      )
    assertEquals(true, Preference.typewriterEnabled.value)
    assertEquals(ThemeMode.Light, Preference.themeMode.value)
    assertEquals(emptyList(), report.importedKeys)
    assertEquals(
      listOf("theme_mode", Preference.TYPEWRITER_ENABLED_KEY).sorted(),
      report.skippedKeys,
    )
  }

  @Test
  fun `import records partial-success state when some keys import and some are skipped`() {
    Preference.devMode.value = true

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
    assertEquals(true, Preference.devMode.value)
  }

  @Test
  fun `import does not reapply handled prefs after user resets them to defaults`() {
    val source =
      LegacyPrefsImportSource(
        preferenceValues = mapOf(Preference.DEV_MODE_KEY to true),
        themeValues = mapOf("mode" to "dark"),
      )

    LegacyPrefsImporter.import(source)
    Preference.devMode.value = Preference.DEFAULT_DEV_MODE
    Preference.themeMode.value = ThemeMode.System

    val report = LegacyPrefsImporter.import(source)

    assertEquals(Preference.DEFAULT_DEV_MODE, Preference.devMode.value)
    assertEquals(ThemeMode.System, Preference.themeMode.value)
    assertEquals(emptyList(), report.importedKeys)
    assertEquals(listOf(Preference.DEV_MODE_KEY, "theme_mode"), report.skippedKeys.sorted())
  }
}
