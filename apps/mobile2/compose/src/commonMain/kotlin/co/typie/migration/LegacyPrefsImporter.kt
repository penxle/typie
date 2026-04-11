package co.typie.migration

import co.typie.service.DeveloperPreferencesService
import co.typie.service.EditorPreferencesService
import co.typie.storage.prefs
import co.typie.ui.theme.ThemeMode

object LegacyPrefsImporter {
  private var siteId: String by prefs("site_id", "")
  private var devMode: Boolean by
    prefs(DeveloperPreferencesService.DEV_MODE_KEY, DeveloperPreferencesService.DEFAULT_DEV_MODE)
  private var typewriterEnabled: Boolean by
    prefs(
      EditorPreferencesService.TYPEWRITER_ENABLED_KEY,
      EditorPreferencesService.DEFAULT_TYPEWRITER_ENABLED,
    )
  private var typewriterPosition: Double by
    prefs(
      EditorPreferencesService.TYPEWRITER_POSITION_KEY,
      EditorPreferencesService.DEFAULT_TYPEWRITER_POSITION,
    )
  private var lineHighlightEnabled: Boolean by
    prefs(
      EditorPreferencesService.LINE_HIGHLIGHT_ENABLED_KEY,
      EditorPreferencesService.DEFAULT_LINE_HIGHLIGHT_ENABLED,
    )
  private var autoSurroundEnabled: Boolean by
    prefs(
      EditorPreferencesService.AUTO_SURROUND_ENABLED_KEY,
      EditorPreferencesService.DEFAULT_AUTO_SURROUND_ENABLED,
    )
  private var characterCountFloatingEnabled: Boolean by
    prefs(
      EditorPreferencesService.CHARACTER_COUNT_FLOATING_ENABLED_KEY,
      EditorPreferencesService.DEFAULT_CHARACTER_COUNT_FLOATING_ENABLED,
    )
  private var widgetAutoFadeEnabled: Boolean by
    prefs(
      EditorPreferencesService.WIDGET_AUTO_FADE_ENABLED_KEY,
      EditorPreferencesService.DEFAULT_WIDGET_AUTO_FADE_ENABLED,
    )
  private var themeMode: ThemeMode by prefs("theme_mode", ThemeMode.System)

  fun import(source: LegacyPrefsImportSource): LegacyPrefsImportReport {
    val importedKeys = mutableListOf<String>()
    val skippedKeys = mutableListOf<String>()

    importString(
      source = source.preferenceValues,
      sourceKey = "site_id",
      targetKey = "site_id",
      currentValue = siteId,
      defaultValue = "",
      setter = { siteId = it },
      importedKeys = importedKeys,
      skippedKeys = skippedKeys,
    )
    importBoolean(
      source = source.preferenceValues,
      sourceKey = DeveloperPreferencesService.DEV_MODE_KEY,
      targetKey = DeveloperPreferencesService.DEV_MODE_KEY,
      currentValue = devMode,
      defaultValue = DeveloperPreferencesService.DEFAULT_DEV_MODE,
      setter = { devMode = it },
      importedKeys = importedKeys,
      skippedKeys = skippedKeys,
    )
    importBoolean(
      source = source.preferenceValues,
      sourceKey = EditorPreferencesService.TYPEWRITER_ENABLED_KEY,
      targetKey = EditorPreferencesService.TYPEWRITER_ENABLED_KEY,
      currentValue = typewriterEnabled,
      defaultValue = EditorPreferencesService.DEFAULT_TYPEWRITER_ENABLED,
      setter = { typewriterEnabled = it },
      importedKeys = importedKeys,
      skippedKeys = skippedKeys,
    )
    importDouble(
      source = source.preferenceValues,
      sourceKey = EditorPreferencesService.TYPEWRITER_POSITION_KEY,
      targetKey = EditorPreferencesService.TYPEWRITER_POSITION_KEY,
      currentValue = typewriterPosition,
      defaultValue = EditorPreferencesService.DEFAULT_TYPEWRITER_POSITION,
      setter = { typewriterPosition = it },
      importedKeys = importedKeys,
      skippedKeys = skippedKeys,
    )
    importBoolean(
      source = source.preferenceValues,
      sourceKey = EditorPreferencesService.LINE_HIGHLIGHT_ENABLED_KEY,
      targetKey = EditorPreferencesService.LINE_HIGHLIGHT_ENABLED_KEY,
      currentValue = lineHighlightEnabled,
      defaultValue = EditorPreferencesService.DEFAULT_LINE_HIGHLIGHT_ENABLED,
      setter = { lineHighlightEnabled = it },
      importedKeys = importedKeys,
      skippedKeys = skippedKeys,
    )
    importBoolean(
      source = source.preferenceValues,
      sourceKey = EditorPreferencesService.AUTO_SURROUND_ENABLED_KEY,
      targetKey = EditorPreferencesService.AUTO_SURROUND_ENABLED_KEY,
      currentValue = autoSurroundEnabled,
      defaultValue = EditorPreferencesService.DEFAULT_AUTO_SURROUND_ENABLED,
      setter = { autoSurroundEnabled = it },
      importedKeys = importedKeys,
      skippedKeys = skippedKeys,
    )
    importBoolean(
      source = source.preferenceValues,
      sourceKey = EditorPreferencesService.CHARACTER_COUNT_FLOATING_ENABLED_KEY,
      targetKey = EditorPreferencesService.CHARACTER_COUNT_FLOATING_ENABLED_KEY,
      currentValue = characterCountFloatingEnabled,
      defaultValue = EditorPreferencesService.DEFAULT_CHARACTER_COUNT_FLOATING_ENABLED,
      setter = { characterCountFloatingEnabled = it },
      importedKeys = importedKeys,
      skippedKeys = skippedKeys,
    )
    importBoolean(
      source = source.preferenceValues,
      sourceKey = EditorPreferencesService.WIDGET_AUTO_FADE_ENABLED_KEY,
      targetKey = EditorPreferencesService.WIDGET_AUTO_FADE_ENABLED_KEY,
      currentValue = widgetAutoFadeEnabled,
      defaultValue = EditorPreferencesService.DEFAULT_WIDGET_AUTO_FADE_ENABLED,
      setter = { widgetAutoFadeEnabled = it },
      importedKeys = importedKeys,
      skippedKeys = skippedKeys,
    )

    val mappedThemeMode = mapLegacyThemeMode(source.themeValues["mode"] as? String)
    if (mappedThemeMode != null) {
      val targetKey = "theme_mode"
      if (LegacyMigrationStateStore.isPrefHandled(targetKey) || themeMode != ThemeMode.System) {
        skippedKeys += targetKey
      } else {
        themeMode = mappedThemeMode
        importedKeys += targetKey
      }
    }

    val report =
      LegacyPrefsImportReport(
        importedKeys = importedKeys.sorted(),
        skippedKeys = skippedKeys.sorted(),
      )
    LegacyMigrationStateStore.recordPrefsImport(report)
    return report
  }

  private fun importString(
    source: Map<String, Any?>,
    sourceKey: String,
    targetKey: String,
    currentValue: String,
    defaultValue: String,
    setter: (String) -> Unit,
    importedKeys: MutableList<String>,
    skippedKeys: MutableList<String>,
  ) {
    val value = source[sourceKey] as? String ?: return
    if (LegacyMigrationStateStore.isPrefHandled(targetKey) || currentValue != defaultValue) {
      skippedKeys += targetKey
      return
    }

    setter(value)
    importedKeys += targetKey
  }

  private fun importBoolean(
    source: Map<String, Any?>,
    sourceKey: String,
    targetKey: String,
    currentValue: Boolean,
    defaultValue: Boolean,
    setter: (Boolean) -> Unit,
    importedKeys: MutableList<String>,
    skippedKeys: MutableList<String>,
  ) {
    val value = source[sourceKey] as? Boolean ?: return
    if (LegacyMigrationStateStore.isPrefHandled(targetKey) || currentValue != defaultValue) {
      skippedKeys += targetKey
      return
    }

    setter(value)
    importedKeys += targetKey
  }

  private fun importDouble(
    source: Map<String, Any?>,
    sourceKey: String,
    targetKey: String,
    currentValue: Double,
    defaultValue: Double,
    setter: (Double) -> Unit,
    importedKeys: MutableList<String>,
    skippedKeys: MutableList<String>,
  ) {
    val value = (source[sourceKey] as? Number)?.toDouble() ?: return
    if (LegacyMigrationStateStore.isPrefHandled(targetKey) || currentValue != defaultValue) {
      skippedKeys += targetKey
      return
    }

    setter(value)
    importedKeys += targetKey
  }
}
