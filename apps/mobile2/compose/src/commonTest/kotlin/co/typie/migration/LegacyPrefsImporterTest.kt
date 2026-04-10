package co.typie.migration

import co.typie.service.DeveloperPreferencesService
import co.typie.service.EditorPreferencesService
import co.typie.storage.Prefs
import co.typie.ui.theme.ThemeMode
import kotlin.test.Test
import kotlin.test.assertEquals

class LegacyPrefsImporterTest {
  @Test
  fun `import maps Flutter dark theme to KMP ThemeMode Dark`() {
    val prefs = createLegacyMigrationTestPrefs()
    val stateStore = LegacyMigrationStateStore(prefs)
    val importer = LegacyPrefsImporter(prefs, stateStore)
    val snapshot = TestPrefsSnapshot(prefs)

    importer.import(
      LegacyPrefsImportSource(preferenceValues = emptyMap(), themeValues = mapOf("mode" to "dark"))
    )

    assertEquals(ThemeMode.Dark, snapshot.themeMode)
  }

  @Test
  fun `import only applies whitelisted keys`() {
    val prefs = createLegacyMigrationTestPrefs()
    val stateStore = LegacyMigrationStateStore(prefs)
    val importer = LegacyPrefsImporter(prefs, stateStore)
    val snapshot = TestPrefsSnapshot(prefs)

    val report =
      importer.import(
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

    assertEquals("site_fixture", snapshot.siteId)
    assertEquals(true, snapshot.devMode)
    assertEquals(ThemeMode.Dark, snapshot.themeMode)
    assertEquals(listOf("dev_mode", "site_id", "theme_mode"), report.importedKeys)
    assertEquals(emptyList(), report.skippedKeys)
  }

  @Test
  fun `import skips keys whose KMP target already has a non-default value`() {
    val prefs = createLegacyMigrationTestPrefs()
    val stateStore = LegacyMigrationStateStore(prefs)
    val importer = LegacyPrefsImporter(prefs, stateStore)
    val snapshot =
      TestPrefsSnapshot(prefs).apply {
        typewriterEnabled = true
        themeMode = ThemeMode.Light
      }

    val report =
      importer.import(
        LegacyPrefsImportSource(
          preferenceValues = mapOf(EditorPreferencesService.TYPEWRITER_ENABLED_KEY to false),
          themeValues = mapOf("mode" to "dark"),
        )
      )

    assertEquals(true, snapshot.typewriterEnabled)
    assertEquals(ThemeMode.Light, snapshot.themeMode)
    assertEquals(emptyList(), report.importedKeys)
    assertEquals(
      listOf("theme_mode", EditorPreferencesService.TYPEWRITER_ENABLED_KEY).sorted(),
      report.skippedKeys,
    )
  }

  @Test
  fun `import records partial-success state when some keys import and some are skipped`() {
    val prefs = createLegacyMigrationTestPrefs()
    val stateStore = LegacyMigrationStateStore(prefs)
    val importer = LegacyPrefsImporter(prefs, stateStore)
    val snapshot = TestPrefsSnapshot(prefs).apply { devMode = true }

    val report =
      importer.import(
        LegacyPrefsImportSource(
          preferenceValues =
            mapOf("site_id" to "site_fixture", DeveloperPreferencesService.DEV_MODE_KEY to false),
          themeValues = mapOf("mode" to "dark"),
        )
      )

    val state = stateStore.snapshot()

    assertEquals(listOf("site_id", "theme_mode"), report.importedKeys)
    assertEquals(listOf(DeveloperPreferencesService.DEV_MODE_KEY), report.skippedKeys)
    assertEquals(LegacyMigrationPhaseStatus.PartiallyImported, state.lastResult)
    assertEquals(listOf("site_id", "theme_mode"), state.importedPrefKeys)
    assertEquals(listOf(DeveloperPreferencesService.DEV_MODE_KEY), state.skippedPrefKeys)
    assertEquals(true, state.importedPrefs)
    assertEquals(true, snapshot.devMode)
  }

  @Test
  fun `import does not reapply handled prefs after user resets them to defaults`() {
    val prefs = createLegacyMigrationTestPrefs()
    val stateStore = LegacyMigrationStateStore(prefs)
    val importer = LegacyPrefsImporter(prefs, stateStore)
    val snapshot = TestPrefsSnapshot(prefs)
    val source =
      LegacyPrefsImportSource(
        preferenceValues = mapOf(DeveloperPreferencesService.DEV_MODE_KEY to true),
        themeValues = mapOf("mode" to "dark"),
      )

    importer.import(source)
    snapshot.devMode = DeveloperPreferencesService.DEFAULT_DEV_MODE
    snapshot.themeMode = ThemeMode.System

    val report = importer.import(source)

    assertEquals(DeveloperPreferencesService.DEFAULT_DEV_MODE, snapshot.devMode)
    assertEquals(ThemeMode.System, snapshot.themeMode)
    assertEquals(emptyList(), report.importedKeys)
    assertEquals(
      listOf(DeveloperPreferencesService.DEV_MODE_KEY, "theme_mode"),
      report.skippedKeys.sorted(),
    )
  }

  private class TestPrefsSnapshot(prefs: Prefs) {
    var siteId: String by prefs("site_id", "")
    var devMode: Boolean by
      prefs(DeveloperPreferencesService.DEV_MODE_KEY, DeveloperPreferencesService.DEFAULT_DEV_MODE)
    var typewriterEnabled: Boolean by
      prefs(
        EditorPreferencesService.TYPEWRITER_ENABLED_KEY,
        EditorPreferencesService.DEFAULT_TYPEWRITER_ENABLED,
      )
    var themeMode: ThemeMode by prefs("theme_mode", ThemeMode.System)
  }
}
