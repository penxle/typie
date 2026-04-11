package co.typie.startup

import co.touchlab.kermit.Logger
import co.typie.auth.AuthService
import co.typie.bootstrap.BootstrapService
import co.typie.migration.LegacyMigrationCoordinator
import kotlinx.coroutines.CancellationException
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.SupervisorJob
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.launch
import kotlinx.coroutines.sync.Mutex
import kotlinx.coroutines.sync.withLock

object AppStartupService {
  private val scope = CoroutineScope(SupervisorJob() + Dispatchers.Default)
  private val mutex = Mutex()
  private var started = false

  private val _state = MutableStateFlow<AppStartupState>(AppStartupState.NotStarted)
  val state: StateFlow<AppStartupState> = _state

  fun startAsync() {
    scope.launch { start() }
  }

  suspend fun start() {
    val shouldStart = mutex.withLock {
      if (started) {
        false
      } else {
        started = true
        true
      }
    }
    if (!shouldStart) return

    Logger.i { "App startup: migration gate entered." }
    _state.value = AppStartupState.Migrating

    val migrationResult =
      try {
        LegacyMigrationCoordinator.runIfNeeded()
      } catch (e: CancellationException) {
        throw e
      } catch (e: Exception) {
        Logger.e(e) { "App startup: migration failed, continuing startup." }
        _state.value = AppStartupState.FailedButContinuing
        null
      }

    Logger.i { "App startup: starting auth and bootstrap services." }
    BootstrapService.start()
    AuthService.renew()
    _state.value = AppStartupState.Ready(migrationResult = migrationResult)
    Logger.i {
      "App startup: ready migration=${migrationResult?.sourceState?.name ?: "FAILED"} auth=${migrationResult?.authResult?.name ?: "FAILED"} prefs=${migrationResult?.prefsResult?.name ?: "FAILED"}."
    }
  }
}
