package co.typie.preflight

import co.typie.Konfig
import co.typie.network.Http
import co.typie.platform.Platform
import co.typie.platform.PlatformModule
import co.typie.storage.Preference
import io.ktor.client.call.body
import io.ktor.client.request.get
import kotlin.time.Instant
import kotlinx.coroutines.CancellationException
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.IO
import kotlinx.coroutines.SupervisorJob
import kotlinx.coroutines.delay
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.launch
import kotlinx.coroutines.sync.Mutex
import kotlinx.coroutines.sync.withLock

private const val PREFLIGHT_CHECK_INTERVAL_MS = 60_000L

object PreflightService {
  private val scope = CoroutineScope(Dispatchers.IO + SupervisorJob())
  private val mutex = Mutex()

  private val _state = MutableStateFlow<PreflightState>(PreflightState.NotReady)
  val state: StateFlow<PreflightState> = _state

  fun launch() {
    if (Preference.preflightCache.value != null) {
      _state.value = PreflightState.from(Preference.preflightCache.value!!)
    }

    scope.launch {
      launch {
        try {
          check()
        } catch (e: Exception) {
          if (_state.value is PreflightState.NotReady) {
            _state.value = PreflightState.Unavailable
          }

          throw e
        }
      }

      while (true) {
        delay(PREFLIGHT_CHECK_INTERVAL_MS)
        try {
          check()
        } catch (e: CancellationException) {
          throw e
        } catch (_: Exception) {
          // best effort
        }
      }
    }
  }

  suspend fun check() = mutex.withLock {
    val preflight = Http.get(Konfig.PREFLIGHT_URL).body<Preflight>()
    Preference.preflightCache.value = preflight
    _state.value = PreflightState.from(preflight)
  }

  fun PreflightState.Companion.from(preflight: Preflight): PreflightState {
    val platform = PlatformModule.platform

    val (platformKey, platformVersion) =
      when (platform) {
        Platform.Android -> "android" to preflight.minVersion.android
        Platform.iOS -> "ios" to preflight.minVersion.ios
        Platform.Desktop -> return PreflightState.Ready
      }

    if (preflight.maintenance.enabled && platformKey in preflight.maintenance.platforms) {
      return PreflightState.UnderMaintenance(
        title = preflight.maintenance.title,
        message = preflight.maintenance.message,
        until = preflight.maintenance.until?.let { Instant.parse(it) },
      )
    }

    val currentVersion = PlatformModule.deviceInfo.retrieve().appVersion
    if (isVersionOlderThan(currentVersion, platformVersion.version)) {
      return PreflightState.UpdateRequired(
        storeUrl = platformVersion.storeUrl,
        currentVersion = currentVersion,
        requiredVersion = platformVersion.version,
      )
    }

    return PreflightState.Ready
  }

  private fun isVersionOlderThan(current: String, required: String): Boolean {
    val currentParts = current.split(".").map { it.toIntOrNull() ?: 0 }
    val requiredParts = required.split(".").map { it.toIntOrNull() ?: 0 }
    val maxLength = maxOf(currentParts.size, requiredParts.size)

    for (i in 0 until maxLength) {
      val c = currentParts.getOrElse(i) { 0 }
      val r = requiredParts.getOrElse(i) { 0 }
      if (c < r) return true
      if (c > r) return false
    }

    return false
  }
}
