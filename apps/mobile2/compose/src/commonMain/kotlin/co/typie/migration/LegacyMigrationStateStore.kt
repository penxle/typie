package co.typie.migration

object LegacyMigrationStateStore {
  //  fun isSessionHandled(): Boolean = Preference.migrationHandledSession.value
  //
  //  fun isPrefHandled(key: String): Boolean =
  //    key in Preference.migrationImportedPrefKeys.value ||
  //      key in Preference.migrationSkippedPrefKeys.value
  //
  //  fun snapshot(): LegacyMigrationState {
  //    val lastResult =
  //      LegacyMigrationPhaseStatus.entries.firstOrNull {
  //        it.name == Preference.migrationLastResultName.value
  //      } ?: LegacyMigrationPhaseStatus.NotStarted
  //
  //    return LegacyMigrationState(
  //      schemaVersion = Preference.migrationSchemaVersion.value,
  //      lastResult = lastResult,
  //      lastAttemptAtMillis = Preference.migrationLastAttemptAtMillis.value,
  //      completedAtMillis = Preference.migrationCompletedAtMillis.value,
  //      importedSession = Preference.migrationImportedSession.value,
  //      importedPrefs = Preference.migrationImportedPrefs.value,
  //      importedPrefKeys = Preference.migrationImportedPrefKeys.value,
  //      skippedPrefKeys = Preference.migrationSkippedPrefKeys.value,
  //    )
  //  }
  //
  //  fun recordPrefsImport(
  //    report: LegacyPrefsImportReport,
  //    nowMillis: Long = Clock.System.now().toEpochMilliseconds(),
  //  ) {
  //    val mergedImportedKeys =
  //      (Preference.migrationImportedPrefKeys.value + report.importedKeys).distinct().sorted()
  //    val mergedSkippedKeys =
  //      (Preference.migrationSkippedPrefKeys.value + report.skippedKeys)
  //        .distinct()
  //        .filterNot(mergedImportedKeys::contains)
  //        .sorted()
  //
  //    Preference.migrationSchemaVersion.value = LEGACY_MIGRATION_SCHEMA_VERSION
  //    Preference.migrationLastAttemptAtMillis.value = nowMillis
  //    Preference.migrationCompletedAtMillis.value = nowMillis
  //    Preference.migrationLastResultName.value = report.status.name
  //    Preference.migrationImportedPrefs.value =
  //      Preference.migrationImportedPrefs.value || report.importedKeys.isNotEmpty()
  //    Preference.migrationImportedPrefKeys.value = mergedImportedKeys
  //    Preference.migrationSkippedPrefKeys.value = mergedSkippedKeys
  //  }
  //
  //  fun recordAuthImported(nowMillis: Long = Clock.System.now().toEpochMilliseconds()) {
  //    Preference.migrationSchemaVersion.value = LEGACY_MIGRATION_SCHEMA_VERSION
  //    Preference.migrationLastAttemptAtMillis.value = nowMillis
  //    Preference.migrationCompletedAtMillis.value = nowMillis
  //    Preference.migrationLastResultName.value = LegacyMigrationPhaseStatus.Imported.name
  //    Preference.migrationHandledSession.value = true
  //    Preference.migrationImportedSession.value = true
  //  }
  //
  //  fun recordAuthSkipped(nowMillis: Long = Clock.System.now().toEpochMilliseconds()) {
  //    Preference.migrationSchemaVersion.value = LEGACY_MIGRATION_SCHEMA_VERSION
  //    Preference.migrationLastAttemptAtMillis.value = nowMillis
  //    Preference.migrationCompletedAtMillis.value = nowMillis
  //    Preference.migrationLastResultName.value = LegacyMigrationPhaseStatus.Skipped.name
  //    Preference.migrationHandledSession.value = true
  //  }
}
