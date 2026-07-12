package co.typie.migration

import co.touchlab.kermit.Logger
import co.typie.domain.auth.AuthService
import co.typie.platform.PlatformModule
import co.typie.storage.Preference
import co.typie.storage.Vault
import kotlinx.coroutines.CancellationException

enum class LegacyAuthMigrationResult {
  AlreadyHandled,
  AlreadyAuthenticated,
  SourceMissing,
  Imported,
  SessionExpired,
  Failed,
}

object LegacyMigrationCoordinator {
  suspend fun runIfNeeded(): LegacyAuthMigrationResult {
    if (Preference.legacyMigrationHandled) {
      return LegacyAuthMigrationResult.AlreadyHandled
    }

    if (Vault.authTokens != null) {
      Preference.legacyMigrationHandled = true
      Logger.i { "Legacy migration: already authenticated, skipping." }
      return LegacyAuthMigrationResult.AlreadyAuthenticated
    }

    val source =
      try {
        PlatformModule.legacyMigrationPlatformSource.load()
      } catch (e: CancellationException) {
        throw e
      } catch (e: Exception) {
        Logger.e(e) { "Legacy migration: failed to load legacy source." }
        null
      }
    if (source == null) {
      Preference.legacyMigrationHandled = true
      Logger.i { "Legacy migration: source missing." }
      return LegacyAuthMigrationResult.SourceMissing
    }

    val sessionToken = runCatching {
      val values =
        LegacyHiveBoxReader.readEncryptedBox(
          bytes = source.bytes,
          keyCrc = source.keyCrc,
          decrypt = source.decryptor::decrypt,
        )
      values["session_token"] as? String ?: error("Missing session_token in legacy auth box.")
    }
      .onFailure { error ->
        Logger.e(error) { "Legacy migration: failed to read legacy auth box." }
      }
      .getOrNull()
    if (sessionToken == null) {
      Preference.legacyMigrationHandled = true
      return LegacyAuthMigrationResult.Failed
    }

    return try {
      AuthService.login(sessionToken)
      Preference.legacyMigrationHandled = true
      Logger.i { "Legacy migration: session imported." }
      LegacyAuthMigrationResult.Imported
    } catch (e: CancellationException) {
      throw e
    } catch (_: AuthService.InvalidCredentialsException) {
      Preference.legacyMigrationHandled = true
      Logger.i { "Legacy migration: legacy session expired." }
      LegacyAuthMigrationResult.SessionExpired
    } catch (e: Exception) {
      Logger.e(e) { "Legacy migration: login failed, will retry on next launch." }
      LegacyAuthMigrationResult.Failed
    }
  }
}
