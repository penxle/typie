package co.typie.migration

object LegacyMigrationStateStore {
  //  fun isSessionHandled(): Boolean = Preference.migrationHandledSession  //
  //  fun isPrefHandled(key: String): Boolean =
  //    key in Preference.migrationImportedPrefKeys ||
  //      key in Preference.migrationSkippedPrefKeys  //
  //  fun snapshot(): LegacyMigrationState {
  //    val lastResult =
  //      LegacyMigrationPhaseStatus.entries.firstOrNull {
  //        it.name == Preference.migrationLastResultName  //      } ?:
  // LegacyMigrationPhaseStatus.NotStarted
  //
  //    return LegacyMigrationState(
  //      schemaVersion = Preference.migrationSchemaVersion,
  //      lastResult = lastResult,
  //      lastAttemptAtMillis = Preference.migrationLastAttemptAtMillis,
  //      completedAtMillis = Preference.migrationCompletedAtMillis,
  //      importedSession = Preference.migrationImportedSession,
  //      importedPrefs = Preference.migrationImportedPrefs,
  //      importedPrefKeys = Preference.migrationImportedPrefKeys,
  //      skippedPrefKeys = Preference.migrationSkippedPrefKeys,
  //    )
  //  }
  //
  //  fun recordPrefsImport(
  //    report: LegacyPrefsImportReport,
  //    nowMillis: Long = Clock.System.now().toEpochMilliseconds(),
  //  ) {
  //    val mergedImportedKeys =
  //      (Preference.migrationImportedPrefKeys + report.importedKeys).distinct().sorted()
  //    val mergedSkippedKeys =
  //      (Preference.migrationSkippedPrefKeys + report.skippedKeys)
  //        .distinct()
  //        .filterNot(mergedImportedKeys::contains)
  //        .sorted()
  //
  //    Preference.migrationSchemaVersion = LEGACY_MIGRATION_SCHEMA_VERSION
  //    Preference.migrationLastAttemptAtMillis = nowMillis
  //    Preference.migrationCompletedAtMillis = nowMillis
  //    Preference.migrationLastResultName = report.status.name
  //    Preference.migrationImportedPrefs =
  //      Preference.migrationImportedPrefs || report.importedKeys.isNotEmpty()
  //    Preference.migrationImportedPrefKeys = mergedImportedKeys
  //    Preference.migrationSkippedPrefKeys = mergedSkippedKeys
  //  }
  //
  //  fun recordAuthImported(nowMillis: Long = Clock.System.now().toEpochMilliseconds()) {
  //    Preference.migrationSchemaVersion = LEGACY_MIGRATION_SCHEMA_VERSION
  //    Preference.migrationLastAttemptAtMillis = nowMillis
  //    Preference.migrationCompletedAtMillis = nowMillis
  //    Preference.migrationLastResultName = LegacyMigrationPhaseStatus.Imported.name
  //    Preference.migrationHandledSession = true
  //    Preference.migrationImportedSession = true
  //  }
  //
  //  fun recordAuthSkipped(nowMillis: Long = Clock.System.now().toEpochMilliseconds()) {
  //    Preference.migrationSchemaVersion = LEGACY_MIGRATION_SCHEMA_VERSION
  //    Preference.migrationLastAttemptAtMillis = nowMillis
  //    Preference.migrationCompletedAtMillis = nowMillis
  //    Preference.migrationLastResultName = LegacyMigrationPhaseStatus.Skipped.name
  //    Preference.migrationHandledSession = true
  //  }
}
