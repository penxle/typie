package co.typie.bootstrap

import co.touchlab.kermit.Logger
import co.typie.Konfig
import co.typie.di.Platform
import co.typie.platform.DeviceInfo
import co.typie.storage.Prefs
import io.ktor.client.HttpClient
import io.ktor.client.call.body
import io.ktor.client.request.get
import kotlinx.coroutines.CancellationException
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.SupervisorJob
import kotlinx.coroutines.delay
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.sync.Mutex
import kotlinx.coroutines.sync.withLock
import kotlinx.coroutines.isActive
import kotlinx.coroutines.launch
import kotlinx.serialization.json.Json
import org.koin.core.annotation.Single

private const val BOOTSTRAP_REFRESH_INTERVAL_MS = 60_000L

@Single(createdAtStart = true)
class BootstrapService(
  private val httpClient: HttpClient,
  private val deviceInfo: DeviceInfo,
  private val platform: Platform,
  prefs: Prefs,
) {
  private val json = Json { ignoreUnknownKeys = true }
  private val scope = CoroutineScope(SupervisorJob() + Dispatchers.Default)
  private val mutex = Mutex()

  private var cachedPayload by prefs("bootstrap_cache", "")

  private val _state = MutableStateFlow<BootstrapState>(BootstrapState.Loading)
  val state: StateFlow<BootstrapState> = _state

  init {
    scope.launch {
      refresh()

      while (isActive) {
        delay(BOOTSTRAP_REFRESH_INTERVAL_MS)
        refresh(showLoading = false)
      }
    }
  }

  suspend fun refresh(showLoading: Boolean = true) = mutex.withLock {
    if (showLoading) {
      _state.value = BootstrapState.Loading
    }

    try {
      val payload = fetchPayload()
      if (payload != null) {
        cachedPayload = payload
        _state.value = resolveBootstrapState(
          bootstrap = json.decodeFromString(BootstrapPayload.serializer(), payload),
          platform = platform,
          currentVersion = deviceInfo.snapshot().appVersion,
        )
        return@withLock
      }
    } catch (e: CancellationException) {
      throw e
    } catch (e: Exception) {
      Logger.e(e) { "Failed to refresh bootstrap" }
    }

    if (cachedPayload.isNotBlank()) {
      try {
        _state.value = resolveBootstrapState(
          bootstrap = json.decodeFromString(BootstrapPayload.serializer(), cachedPayload),
          platform = platform,
          currentVersion = deviceInfo.snapshot().appVersion,
        )
        return@withLock
      } catch (e: CancellationException) {
        throw e
      } catch (e: Exception) {
        Logger.e(e) { "Failed to decode cached bootstrap" }
      }
    }

    _state.value = BootstrapState.Ready
  }

  private suspend fun fetchPayload(): String? {
    return runCatching {
      httpClient.get(bootstrapUrlForApiUrl(Konfig.API_URL)).body<String>()
    }.getOrNull()
  }
}
