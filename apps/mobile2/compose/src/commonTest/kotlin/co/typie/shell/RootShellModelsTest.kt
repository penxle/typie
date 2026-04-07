package co.typie.shell

import co.typie.auth.AuthState
import co.typie.bootstrap.BootstrapState
import co.typie.migration.LegacyMigrationRunResult
import co.typie.migration.LegacyMigrationSourceState
import co.typie.migration.LegacyMigrationStepResult
import co.typie.startup.AppStartupState
import kotlin.test.Test
import kotlin.test.assertEquals

class RootShellModelsTest {
  @Test
  fun `rootShellTargetState resolves authenticated destinations without session token state`() {
    assertEquals(
      RootShellTargetState(RootShellDestination.Main),
      rootShellTargetState(AppStartupState.Ready(readyMigrationResult()), AuthState.Authenticated, BootstrapState.Ready),
    )
  }

  @Test
  fun `rootShellTargetState ignores stale session context outside authenticated state`() {
    assertEquals(
      RootShellTargetState(RootShellDestination.Auth),
      rootShellTargetState(AppStartupState.Ready(readyMigrationResult()), AuthState.Unauthenticated, BootstrapState.Ready),
    )
  }

  @Test
  fun `resolveRootShellDestination prioritizes bootstrap blockers before auth destinations`() {
    assertEquals(
      RootShellDestination.Maintenance(
        title = "점검 중",
        message = "잠시 후 다시 시도해주세요.",
        until = null,
      ),
      resolveRootShellDestination(
        startupState = AppStartupState.Ready(readyMigrationResult()),
        authState = AuthState.Authenticated,
        bootstrapState = BootstrapState.Maintenance(
          title = "점검 중",
          message = "잠시 후 다시 시도해주세요.",
          until = null,
        ),
      ),
    )
    assertEquals(
      RootShellDestination.Offline,
      resolveRootShellDestination(
        startupState = AppStartupState.Ready(readyMigrationResult()),
        authState = AuthState.Offline,
        bootstrapState = BootstrapState.Ready,
      ),
    )
  }

  @Test
  fun `startup state keeps root shell on splash before auth bootstrap resolution begins`() {
    assertEquals(
      RootShellDestination.Splash,
      resolveRootShellDestination(
        startupState = AppStartupState.NotStarted,
        authState = AuthState.Authenticated,
        bootstrapState = BootstrapState.Ready,
      ),
    )
    assertEquals(
      RootShellDestination.Splash,
      resolveRootShellDestination(
        startupState = AppStartupState.Migrating,
        authState = AuthState.Authenticated,
        bootstrapState = BootstrapState.Ready,
      ),
    )
  }

  private fun readyMigrationResult(): LegacyMigrationRunResult {
    return LegacyMigrationRunResult(
      sourceState = LegacyMigrationSourceState.Missing,
      authResult = LegacyMigrationStepResult.NotAttempted,
      prefsResult = LegacyMigrationStepResult.NotAttempted,
    )
  }
}
