package co.typie.startup

import co.typie.migration.LegacyMigrationRunResult

fun interface AuthStartupHandle {
  suspend fun start()
}

fun interface BootstrapStartupHandle {
  suspend fun start()
}

sealed interface AppStartupState {
  data object NotStarted : AppStartupState
  data object Migrating : AppStartupState
  data object FailedButContinuing : AppStartupState
  data class Ready(
    val migrationResult: LegacyMigrationRunResult?,
  ) : AppStartupState
}
