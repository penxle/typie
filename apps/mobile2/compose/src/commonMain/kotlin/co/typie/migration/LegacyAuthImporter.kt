package co.typie.migration

import co.typie.storage.vault

object LegacyAuthImporter {
  private var tokens: AuthTokens? by vault("tokens", null)

  fun importSessionToken(sessionToken: String): LegacyMigrationStepResult {
    if (LegacyMigrationStateStore.isSessionHandled()) {
      return LegacyMigrationStepResult.Skipped
    }

    if (tokens != null) {
      LegacyMigrationStateStore.recordAuthSkipped()
      return LegacyMigrationStepResult.Skipped
    }

    tokens = AuthTokens(sessionToken = sessionToken)
    LegacyMigrationStateStore.recordAuthImported()
    return LegacyMigrationStepResult.Imported
  }
}
