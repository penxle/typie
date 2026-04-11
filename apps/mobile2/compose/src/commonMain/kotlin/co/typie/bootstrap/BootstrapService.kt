package co.typie.bootstrap

import co.touchlab.kermit.Logger
import co.typie.Konfig
import co.typie.graphql.Http
import co.typie.platform.PlatformModule
import co.typie.startup.BootstrapStartupHandle
import co.typie.storage.prefs
import io.ktor.client.call.body
import io.ktor.client.request.get
import kotlinx.coroutines.CancellationException
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.Job
import kotlinx.coroutines.SupervisorJob
import kotlinx.coroutines.delay
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.isActive
import kotlinx.coroutines.launch
import kotlinx.coroutines.sync.Mutex
import kotlinx.coroutines.sync.withLock
import kotlinx.serialization.json.Json

private const val BOOTSTRAP_REFRESH_INTERVAL_MS = 60_000L

object BootstrapService : BootstrapStartupHandle {
  private val json = Json { ignoreUnknownKeys = true }
  private val scope = CoroutineScope(SupervisorJob() + Dispatchers.Default)
  private val mutex = Mutex()
  private var started = false
  private var refreshLoopJob: Job? = null

  private var cachedPayload by prefs("bootstrap_cache", "")

  private val _state = MutableStateFlow<BootstrapState>(BootstrapState.Loading)
  val state: StateFlow<BootstrapState> = _state

  override suspend fun start() {
    val shouldStart = mutex.withLock {
      if (started) {
        false
      } else {
        started = true
        true
      }
    }
    if (!shouldStart) return

    Logger.i { "Bootstrap startup: begin initial refresh." }
    refresh()
    refreshLoopJob = scope.launch {
      while (isActive) {
        delay(BOOTSTRAP_REFRESH_INTERVAL_MS)
        refresh(showLoading = false)
      }
    }
  }

  fun startAsync() {
    scope.launch { start() }
  }

  suspend fun refresh(showLoading: Boolean = true) = mutex.withLock {
    if (showLoading) {
      _state.value = BootstrapState.Loading
    }

    try {
      val payload = fetchPayload()
      if (payload != null) {
        cachedPayload = payload
        _state.value =
            resolveBootstrapState(
                bootstrap = json.decodeFromString(BootstrapPayload.serializer(), payload),
                platform = PlatformModule.platform,
                currentVersion = PlatformModule.deviceInfo.snapshot().appVersion,
            )
        Logger.i { "Bootstrap startup: loaded remote bootstrap." }
        return@withLock
      }
    } catch (e: CancellationException) {
      throw e
    } catch (e: Exception) {
      Logger.e(e) { "Failed to refresh bootstrap" }
    }

    if (cachedPayload.isNotBlank()) {
      try {
        _state.value =
            resolveBootstrapState(
                bootstrap = json.decodeFromString(BootstrapPayload.serializer(), cachedPayload),
                platform = PlatformModule.platform,
                currentVersion = PlatformModule.deviceInfo.snapshot().appVersion,
            )
        Logger.i { "Bootstrap startup: loaded cached bootstrap." }
        return@withLock
      } catch (e: CancellationException) {
        throw e
      } catch (e: Exception) {
        Logger.e(e) { "Failed to decode cached bootstrap" }
      }
    }

    _state.value = BootstrapState.Ready
    Logger.i { "Bootstrap startup: continuing without bootstrap payload." }
  }

  private suspend fun fetchPayload(): String? {
    return runCatching { Http.get(bootstrapUrlForApiUrl(Konfig.API_URL)).body<String>() }
        .getOrNull()
  }
}
