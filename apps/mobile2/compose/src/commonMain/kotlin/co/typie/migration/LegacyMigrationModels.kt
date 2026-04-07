package co.typie.migration

import co.typie.ui.theme.ThemeMode

const val LEGACY_MIGRATION_SCHEMA_VERSION = 1

enum class LegacyMigrationPhaseStatus {
  NotStarted,
  Imported,
  PartiallyImported,
  Skipped,
  Failed,
}

enum class LegacyMigrationSourceState {
  Missing,
  Available,
}

enum class LegacyMigrationStepResult {
  NotAttempted,
  Imported,
  Skipped,
  Failed,
}

data class LegacyPrefsImportSource(
  val preferenceValues: Map<String, Any?>,
  val themeValues: Map<String, Any?>,
)

data class LegacyPrefsImportReport(
  val importedKeys: List<String>,
  val skippedKeys: List<String>,
) {
  val status: LegacyMigrationPhaseStatus =
    when {
      importedKeys.isNotEmpty() && skippedKeys.isNotEmpty() -> LegacyMigrationPhaseStatus.PartiallyImported
      importedKeys.isNotEmpty() -> LegacyMigrationPhaseStatus.Imported
      skippedKeys.isNotEmpty() -> LegacyMigrationPhaseStatus.Skipped
      else -> LegacyMigrationPhaseStatus.NotStarted
    }
}

data class LegacyMigrationState(
  val schemaVersion: Int,
  val lastResult: LegacyMigrationPhaseStatus,
  val lastAttemptAtMillis: Long,
  val completedAtMillis: Long,
  val importedSession: Boolean,
  val importedPrefs: Boolean,
  val importedPrefKeys: List<String>,
  val skippedPrefKeys: List<String>,
)

data class LegacyMigrationRunResult(
  val sourceState: LegacyMigrationSourceState,
  val authResult: LegacyMigrationStepResult,
  val prefsResult: LegacyMigrationStepResult,
)

internal fun mapLegacyThemeMode(value: String?): ThemeMode? {
  return when (value?.lowercase()) {
    "system" -> ThemeMode.System
    "light" -> ThemeMode.Light
    "dark" -> ThemeMode.Dark
    else -> null
  }
}
