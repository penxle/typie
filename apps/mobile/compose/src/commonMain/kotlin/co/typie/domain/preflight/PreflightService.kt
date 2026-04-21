package co.typie.domain.preflight

import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.setValue
import co.typie.Konfig
import co.typie.network.Http
import co.typie.platform.Platform
import co.typie.platform.PlatformModule
import co.typie.storage.Preference
import io.ktor.client.call.body
import io.ktor.client.request.get
import io.sentry.kotlin.multiplatform.Sentry
import kotlin.time.Instant
import kotlinx.coroutines.CancellationException
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.IO
import kotlinx.coroutines.SupervisorJob
import kotlinx.coroutines.delay
import kotlinx.coroutines.launch
import kotlinx.coroutines.sync.Mutex
import kotlinx.coroutines.sync.withLock

private const val PREFLIGHT_CHECK_INTERVAL_MS = 60_000L

object PreflightService {
  private val scope = CoroutineScope(Dispatchers.IO + SupervisorJob())
  private val mutex = Mutex()

  var state by mutableStateOf<PreflightState>(PreflightState.NotReady)
    private set

  fun launch() {
    if (Preference.preflightCache != null) {
      state = PreflightState.from(Preference.preflightCache!!)
    }

    scope.launch {
      launch {
        try {
          check()
        } catch (e: CancellationException) {
          throw e
        } catch (e: Exception) {
          Sentry.captureException(e)
          if (state is PreflightState.NotReady) {
            state = PreflightState.Unavailable
          }
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
    Preference.preflightCache = preflight
    state = PreflightState.from(preflight)
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
