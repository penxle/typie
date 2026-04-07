package co.typie.migration

import co.typie.auth.AuthTokens
import co.typie.storage.Vault
import org.koin.core.annotation.Single

@Single
class LegacyAuthImporter(
  vault: Vault,
  private val stateStore: LegacyMigrationStateStore,
) {
  private var tokens: AuthTokens? by vault("tokens", null)

  fun importSessionToken(sessionToken: String): LegacyMigrationStepResult {
    if (stateStore.isSessionHandled()) {
      return LegacyMigrationStepResult.Skipped
    }

    if (tokens != null) {
      stateStore.recordAuthSkipped()
      return LegacyMigrationStepResult.Skipped
    }

    tokens = AuthTokens(sessionToken = sessionToken)
    stateStore.recordAuthImported()
    return LegacyMigrationStepResult.Imported
  }
}
