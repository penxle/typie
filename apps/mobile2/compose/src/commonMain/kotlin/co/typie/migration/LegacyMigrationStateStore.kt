package co.typie.migration

import co.typie.storage.prefs
import kotlin.time.Clock

object LegacyMigrationStateStore {
  private var schemaVersion: Int by prefs("legacy_migration_schema_version", 0)
  private var lastResultName: String by
      prefs("legacy_migration_last_result", LegacyMigrationPhaseStatus.NotStarted.name)
  private var lastAttemptAtMillis: Long by prefs("legacy_migration_last_attempt_at", 0L)
  private var completedAtMillis: Long by prefs("legacy_migration_completed_at", 0L)
  private var handledSession: Boolean by prefs("legacy_migration_handled_session", false)
  private var importedSession: Boolean by prefs("legacy_migration_imported_session", false)
  private var importedPrefs: Boolean by prefs("legacy_migration_imported_prefs", false)
  private var importedPrefKeys: List<String> by
      prefs("legacy_migration_imported_pref_keys", emptyList<String>())
  private var skippedPrefKeys: List<String> by
      prefs("legacy_migration_skipped_pref_keys", emptyList<String>())

  fun isSessionHandled(): Boolean = handledSession

  fun isPrefHandled(key: String): Boolean = key in importedPrefKeys || key in skippedPrefKeys

  fun snapshot(): LegacyMigrationState {
    val lastResult =
        LegacyMigrationPhaseStatus.entries.firstOrNull { it.name == lastResultName }
            ?: LegacyMigrationPhaseStatus.NotStarted

    return LegacyMigrationState(
        schemaVersion = schemaVersion,
        lastResult = lastResult,
        lastAttemptAtMillis = lastAttemptAtMillis,
        completedAtMillis = completedAtMillis,
        importedSession = importedSession,
        importedPrefs = importedPrefs,
        importedPrefKeys = importedPrefKeys,
        skippedPrefKeys = skippedPrefKeys,
    )
  }

  fun recordPrefsImport(
      report: LegacyPrefsImportReport,
      nowMillis: Long = Clock.System.now().toEpochMilliseconds(),
  ) {
    val mergedImportedKeys = (importedPrefKeys + report.importedKeys).distinct().sorted()
    val mergedSkippedKeys =
        (skippedPrefKeys + report.skippedKeys)
            .distinct()
            .filterNot(mergedImportedKeys::contains)
            .sorted()

    schemaVersion = LEGACY_MIGRATION_SCHEMA_VERSION
    lastAttemptAtMillis = nowMillis
    completedAtMillis = nowMillis
    lastResultName = report.status.name
    importedPrefs = importedPrefs || report.importedKeys.isNotEmpty()
    importedPrefKeys = mergedImportedKeys
    skippedPrefKeys = mergedSkippedKeys
  }

  fun recordAuthImported(nowMillis: Long = Clock.System.now().toEpochMilliseconds()) {
    schemaVersion = LEGACY_MIGRATION_SCHEMA_VERSION
    lastAttemptAtMillis = nowMillis
    completedAtMillis = nowMillis
    lastResultName = LegacyMigrationPhaseStatus.Imported.name
    handledSession = true
    importedSession = true
  }

  fun recordAuthSkipped(nowMillis: Long = Clock.System.now().toEpochMilliseconds()) {
    schemaVersion = LEGACY_MIGRATION_SCHEMA_VERSION
    lastAttemptAtMillis = nowMillis
    completedAtMillis = nowMillis
    lastResultName = LegacyMigrationPhaseStatus.Skipped.name
    handledSession = true
  }
}
