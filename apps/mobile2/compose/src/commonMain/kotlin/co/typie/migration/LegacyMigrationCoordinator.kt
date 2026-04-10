package co.typie.migration

import co.touchlab.kermit.Logger
import co.typie.platform.PlatformModule

fun interface LegacyMigrationRunner {
  suspend fun runIfNeeded(): LegacyMigrationRunResult
}

object LegacyMigrationCoordinator : LegacyMigrationRunner {
  override suspend fun runIfNeeded(): LegacyMigrationRunResult {
    Logger.i { "Legacy migration: checking legacy source." }
    val source = PlatformModule.legacyMigrationPlatformSource.load()
      ?: return LegacyMigrationRunResult(
        sourceState = LegacyMigrationSourceState.Missing,
        authResult = LegacyMigrationStepResult.NotAttempted,
        prefsResult = LegacyMigrationStepResult.NotAttempted,
      ).also {
        Logger.i { "Legacy migration: source missing." }
      }

    val authResult = when {
      LegacyMigrationStateStore.isSessionHandled() -> LegacyMigrationStepResult.Skipped
      else -> source.authBox?.let(::importAuth).orElseNotAttempted()
    }
    val prefsResult = takeIf { source.preferenceBox != null || source.themeBox != null }
      ?.let { importPrefs(source) }
      .orElseNotAttempted()

    return LegacyMigrationRunResult(
      sourceState = LegacyMigrationSourceState.Available,
      authResult = authResult,
      prefsResult = prefsResult,
    ).also { result ->
      Logger.i {
        "Legacy migration: completed source=${result.sourceState.name} auth=${result.authResult.name} prefs=${result.prefsResult.name}."
      }
    }
  }

  private fun importAuth(source: LegacyEncryptedHiveBoxSource): LegacyMigrationStepResult {
    return runCatching {
      val authValues = LegacyHiveBoxReader.readEncryptedBox(
        bytes = source.bytes,
        keyCrc = source.keyCrc,
        decrypt = source.decryptor::decrypt,
      )
      val sessionToken = authValues["session_token"] as? String ?: error("Missing session_token in legacy auth box.")
      LegacyAuthImporter.importSessionToken(sessionToken)
    }.onFailure { error ->
      Logger.e(error) {
        "Legacy migration: auth import failed (${error::class.simpleName}): ${error.message ?: "<no message>"}."
      }
    }.getOrElse { LegacyMigrationStepResult.Failed }
  }

  private fun importPrefs(source: LegacyMigrationSource): LegacyMigrationStepResult {
    return runCatching {
      val preferenceValues = source.preferenceBox?.let(LegacyHiveBoxReader::readBox).orEmpty()
      val themeValues = source.themeBox?.let(LegacyHiveBoxReader::readBox).orEmpty()
      val report = LegacyPrefsImporter.import(
        LegacyPrefsImportSource(
          preferenceValues = preferenceValues,
          themeValues = themeValues,
        ),
      )
      Logger.i {
        "Legacy migration: prefs imported=${report.importedKeys.size} skipped=${report.skippedKeys.size}."
      }
      when (report.status) {
        LegacyMigrationPhaseStatus.Imported,
        LegacyMigrationPhaseStatus.PartiallyImported,
        -> LegacyMigrationStepResult.Imported

        LegacyMigrationPhaseStatus.Skipped -> LegacyMigrationStepResult.Skipped
        LegacyMigrationPhaseStatus.NotStarted -> LegacyMigrationStepResult.NotAttempted
        LegacyMigrationPhaseStatus.Failed -> LegacyMigrationStepResult.Failed
      }
    }.onFailure { error ->
      Logger.e(error) {
        "Legacy migration: prefs import failed (${error::class.simpleName}): ${error.message ?: "<no message>"}."
      }
    }.getOrElse { LegacyMigrationStepResult.Failed }
  }
}

private fun LegacyMigrationStepResult?.orElseNotAttempted(): LegacyMigrationStepResult {
  return this ?: LegacyMigrationStepResult.NotAttempted
}
