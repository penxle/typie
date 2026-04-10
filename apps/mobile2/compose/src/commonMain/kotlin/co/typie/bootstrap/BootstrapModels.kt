package co.typie.bootstrap

import co.typie.datetime.toInstantOrNull
import co.typie.platform.Platform
import io.ktor.http.Url
import kotlin.time.Instant
import kotlinx.serialization.Serializable

@Serializable
data class BootstrapPayload(
  val version: Int,
  val updatedAt: String,
  val maintenance: BootstrapMaintenancePayload,
  val minVersion: BootstrapMinVersionPayload,
)

@Serializable
data class BootstrapMaintenancePayload(
  val enabled: Boolean,
  val title: String,
  val message: String,
  val until: String? = null,
  val platforms: List<String> = emptyList(),
)

@Serializable
data class BootstrapMinVersionPayload(
  val ios: BootstrapPlatformMinVersionPayload,
  val android: BootstrapPlatformMinVersionPayload,
)

@Serializable
data class BootstrapPlatformMinVersionPayload(val version: String, val storeUrl: String)

sealed interface BootstrapState {
  data object Loading : BootstrapState

  data object Ready : BootstrapState

  data class Maintenance(val title: String, val message: String, val until: Instant?) :
    BootstrapState

  data class UpdateRequired(
    val storeUrl: String,
    val currentVersion: String,
    val requiredVersion: String,
  ) : BootstrapState
}

fun bootstrapEnvironmentForApiUrl(apiUrl: String): String {
  val host =
    runCatching { Url(apiUrl).host.lowercase() }
      .getOrElse {
        return "prod"
      }

  return when {
    host == "localhost" || host == "127.0.0.1" -> "local"
    host.startsWith("api.dev.") || ".dev." in host -> "dev"
    else -> "prod"
  }
}

fun bootstrapUrlForApiUrl(apiUrl: String): String {
  return "https://config.typie.net/bootstrap/${bootstrapEnvironmentForApiUrl(apiUrl)}.json"
}

fun normalizeBootstrapVersion(value: String): String {
  return value.substringBefore(' ').trim()
}

fun isBootstrapVersionLower(current: String, required: String): Boolean {
  val currentParts =
    normalizeBootstrapVersion(current).split('.').map { it.toIntOrNull() ?: 0 }.toMutableList()
  val requiredParts =
    normalizeBootstrapVersion(required).split('.').map { it.toIntOrNull() ?: 0 }.toMutableList()
  val maxSize = maxOf(currentParts.size, requiredParts.size)

  while (currentParts.size < maxSize) {
    currentParts += 0
  }

  while (requiredParts.size < maxSize) {
    requiredParts += 0
  }

  for (index in 0 until maxSize) {
    when {
      currentParts[index] < requiredParts[index] -> return true
      currentParts[index] > requiredParts[index] -> return false
    }
  }

  return false
}

fun resolveBootstrapState(
  bootstrap: BootstrapPayload,
  platform: Platform,
  currentVersion: String,
): BootstrapState {
  val platformKey =
    when (platform) {
      Platform.Android -> "android"
      Platform.iOS -> "ios"
      Platform.Desktop -> return BootstrapState.Ready
    }

  if (bootstrap.maintenance.enabled && platformKey in bootstrap.maintenance.platforms) {
    return BootstrapState.Maintenance(
      title = bootstrap.maintenance.title,
      message = bootstrap.maintenance.message,
      until = bootstrap.maintenance.until?.toInstantOrNull(),
    )
  }

  val minVersion =
    when (platform) {
      Platform.Android -> bootstrap.minVersion.android
      Platform.iOS -> bootstrap.minVersion.ios
      Platform.Desktop -> return BootstrapState.Ready
    }

  if (isBootstrapVersionLower(current = currentVersion, required = minVersion.version)) {
    return BootstrapState.UpdateRequired(
      storeUrl = minVersion.storeUrl,
      currentVersion = normalizeBootstrapVersion(currentVersion),
      requiredVersion = normalizeBootstrapVersion(minVersion.version),
    )
  }

  return BootstrapState.Ready
}
