package co.typie.startup

import co.typie.migration.LegacyMigrationRunResult
import co.typie.migration.LegacyMigrationRunner
import co.typie.migration.LegacyMigrationSourceState
import co.typie.migration.LegacyMigrationStepResult
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlinx.coroutines.ExperimentalCoroutinesApi
import kotlinx.coroutines.test.runTest

@OptIn(ExperimentalCoroutinesApi::class)
class AppStartupServiceTest {
  @Test
  fun `migration runs before auth and bootstrap start`() = runTest {
    val order = mutableListOf<String>()
    val service =
      AppStartupService(
        migrationRunner =
          LegacyMigrationRunner {
            order += "migration"
            LegacyMigrationRunResult(
              sourceState = LegacyMigrationSourceState.Missing,
              authResult = LegacyMigrationStepResult.NotAttempted,
              prefsResult = LegacyMigrationStepResult.NotAttempted,
            )
          },
        authStartupHandle = AuthStartupHandle { order += "auth" },
        bootstrapStartupHandle = BootstrapStartupHandle { order += "bootstrap" },
      )

    service.start()

    assertEquals(listOf("migration", "auth", "bootstrap"), order)
    assertEquals(
      AppStartupState.Ready(
        migrationResult =
          LegacyMigrationRunResult(
            sourceState = LegacyMigrationSourceState.Missing,
            authResult = LegacyMigrationStepResult.NotAttempted,
            prefsResult = LegacyMigrationStepResult.NotAttempted,
          )
      ),
      service.state.value,
    )
  }

  @Test
  fun `failedbutcontinuing still transitions into service startup`() = runTest {
    val order = mutableListOf<String>()
    lateinit var service: AppStartupService
    var authObservedState: AppStartupState? = null

    service =
      AppStartupService(
        migrationRunner =
          LegacyMigrationRunner {
            order += "migration"
            error("boom")
          },
        authStartupHandle =
          AuthStartupHandle {
            authObservedState = service.state.value
            order += "auth"
          },
        bootstrapStartupHandle = BootstrapStartupHandle { order += "bootstrap" },
      )

    service.start()

    assertEquals(listOf("migration", "auth", "bootstrap"), order)
    assertEquals(AppStartupState.FailedButContinuing, authObservedState)
    assertEquals(AppStartupState.Ready(migrationResult = null), service.state.value)
  }
}
