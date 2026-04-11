package co.typie.migration

object LegacyAuthImporter {
  fun importSessionToken(sessionToken: String): LegacyMigrationStepResult {
    //    if (LegacyMigrationStateStore.isSessionHandled()) {
    //      return LegacyMigrationStepResult.Skipped
    //    }

    //    if (Vault.legacyTokens != null) {
    //      LegacyMigrationStateStore.recordAuthSkipped()
    //      return LegacyMigrationStepResult.Skipped
    //    }
    //
    //    Vault.legacyTokens = AuthTokens(sessionToken = sessionToken)
    //    LegacyMigrationStateStore.recordAuthImported()
    return LegacyMigrationStepResult.Imported
  }
}
