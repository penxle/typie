package co.typie.migration

object LegacyAuthImporter {
  fun importSessionToken(sessionToken: String): LegacyMigrationStepResult {
    //    if (LegacyMigrationStateStore.isSessionHandled()) {
    //      return LegacyMigrationStepResult.Skipped
    //    }

    //    if (Vault.legacyTokens.value != null) {
    //      LegacyMigrationStateStore.recordAuthSkipped()
    //      return LegacyMigrationStepResult.Skipped
    //    }
    //
    //    Vault.legacyTokens.value = AuthTokens(sessionToken = sessionToken)
    //    LegacyMigrationStateStore.recordAuthImported()
    return LegacyMigrationStepResult.Imported
  }
}
