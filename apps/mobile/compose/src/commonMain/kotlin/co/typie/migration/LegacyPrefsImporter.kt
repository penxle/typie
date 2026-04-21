package co.typie.migration

object LegacyPrefsImporter {
  fun import(source: LegacyPrefsImportSource): LegacyPrefsImportReport {
    val importedKeys = mutableListOf<String>()
    val skippedKeys = mutableListOf<String>()

    //    importString(
    //      source = source.preferenceValues,
    //      sourceKey = "site_id",
    //      targetKey = "site_id",
    //      currentValue = Preference.legacySiteId,
    //      defaultValue = "",
    //      setter = { Preference.legacySiteId = it },
    //      importedKeys = importedKeys,
    //      skippedKeys = skippedKeys,
    //    )
    //    importBoolean(
    //      source = source.preferenceValues,
    //      sourceKey = Preference.DEV_MODE_KEY,
    //      targetKey = Preference.DEV_MODE_KEY,
    //      currentValue = Preference.devMode,
    //      defaultValue = Preference.DEFAULT_DEV_MODE,
    //      setter = { Preference.devMode = it },
    //      importedKeys = importedKeys,
    //      skippedKeys = skippedKeys,
    //    )
    //    importBoolean(
    //      source = source.preferenceValues,
    //      sourceKey = Preference.TYPEWRITER_ENABLED_KEY,
    //      targetKey = Preference.TYPEWRITER_ENABLED_KEY,
    //      currentValue = Preference.typewriterEnabled,
    //      defaultValue = Preference.DEFAULT_TYPEWRITER_ENABLED,
    //      setter = { Preference.typewriterEnabled = it },
    //      importedKeys = importedKeys,
    //      skippedKeys = skippedKeys,
    //    )
    //    importDouble(
    //      source = source.preferenceValues,
    //      sourceKey = Preference.TYPEWRITER_POSITION_KEY,
    //      targetKey = Preference.TYPEWRITER_POSITION_KEY,
    //      currentValue = Preference.typewriterPosition,
    //      defaultValue = Preference.DEFAULT_TYPEWRITER_POSITION,
    //      setter = { Preference.typewriterPosition = it },
    //      importedKeys = importedKeys,
    //      skippedKeys = skippedKeys,
    //    )
    //    importBoolean(
    //      source = source.preferenceValues,
    //      sourceKey = Preference.LINE_HIGHLIGHT_ENABLED_KEY,
    //      targetKey = Preference.LINE_HIGHLIGHT_ENABLED_KEY,
    //      currentValue = Preference.lineHighlightEnabled,
    //      defaultValue = Preference.DEFAULT_LINE_HIGHLIGHT_ENABLED,
    //      setter = { Preference.lineHighlightEnabled = it },
    //      importedKeys = importedKeys,
    //      skippedKeys = skippedKeys,
    //    )
    //    importBoolean(
    //      source = source.preferenceValues,
    //      sourceKey = Preference.AUTO_SURROUND_ENABLED_KEY,
    //      targetKey = Preference.AUTO_SURROUND_ENABLED_KEY,
    //      currentValue = Preference.autoSurroundEnabled,
    //      defaultValue = Preference.DEFAULT_AUTO_SURROUND_ENABLED,
    //      setter = { Preference.autoSurroundEnabled = it },
    //      importedKeys = importedKeys,
    //      skippedKeys = skippedKeys,
    //    )
    //    importBoolean(
    //      source = source.preferenceValues,
    //      sourceKey = Preference.CHARACTER_COUNT_FLOATING_ENABLED_KEY,
    //      targetKey = Preference.CHARACTER_COUNT_FLOATING_ENABLED_KEY,
    //      currentValue = Preference.characterCountFloatingEnabled,
    //      defaultValue = Preference.DEFAULT_CHARACTER_COUNT_FLOATING_ENABLED,
    //      setter = { Preference.characterCountFloatingEnabled = it },
    //      importedKeys = importedKeys,
    //      skippedKeys = skippedKeys,
    //    )
    //    importBoolean(
    //      source = source.preferenceValues,
    //      sourceKey = Preference.WIDGET_AUTO_FADE_ENABLED_KEY,
    //      targetKey = Preference.WIDGET_AUTO_FADE_ENABLED_KEY,
    //      currentValue = Preference.widgetAutoFadeEnabled,
    //      defaultValue = Preference.DEFAULT_WIDGET_AUTO_FADE_ENABLED,
    //      setter = { Preference.widgetAutoFadeEnabled = it },
    //      importedKeys = importedKeys,
    //      skippedKeys = skippedKeys,
    //    )

    val mappedThemeMode = mapLegacyThemeMode(source.themeValues["mode"] as? String)
    if (mappedThemeMode != null) {
      val targetKey = "theme_mode"
      //      if (
      //        LegacyMigrationStateStore.isPrefHandled(targetKey) ||
      //          Preference.themeMode != ThemeMode.System
      //      ) {
      //        skippedKeys += targetKey
      //      } else {
      //        Preference.themeMode = mappedThemeMode
      //        importedKeys += targetKey
      //      }
    }

    val report =
      LegacyPrefsImportReport(
        importedKeys = importedKeys.sorted(),
        skippedKeys = skippedKeys.sorted(),
      )
    //    LegacyMigrationStateStore.recordPrefsImport(report)
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
    //    if (LegacyMigrationStateStore.isPrefHandled(targetKey) || currentValue != defaultValue) {
    //      skippedKeys += targetKey
    //      return
    //    }

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
    //    if (LegacyMigrationStateStore.isPrefHandled(targetKey) || currentValue != defaultValue) {
    //      skippedKeys += targetKey
    //      return
    //    }

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
    //    if (LegacyMigrationStateStore.isPrefHandled(targetKey) || currentValue != defaultValue) {
    //      skippedKeys += targetKey
    //      return
    //    }

    setter(value)
    importedKeys += targetKey
  }
}
