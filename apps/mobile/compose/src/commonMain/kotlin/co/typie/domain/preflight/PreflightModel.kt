package co.typie.domain.preflight

import kotlin.time.Instant
import kotlinx.serialization.Serializable

sealed interface PreflightState {
  data object NotReady : PreflightState

  data class UnderMaintenance(val title: String, val message: String, val until: Instant?) :
    PreflightState

  data class UpdateRequired(
    val storeUrl: String,
    val currentVersion: String,
    val requiredVersion: String,
  ) : PreflightState

  data object Ready : PreflightState

  data object Unavailable : PreflightState

  companion object
}

@Serializable
data class Preflight(val maintenance: PreflightMaintenance, val minVersion: PreflightMinVersion)

@Serializable
data class PreflightMaintenance(
  val enabled: Boolean,
  val title: String,
  val message: String,
  val until: String? = null,
  val platforms: List<String> = emptyList(),
)

@Serializable
data class PreflightMinVersion(
  val ios: PreflightMinVersionPlatform,
  val android: PreflightMinVersionPlatform,
)

@Serializable data class PreflightMinVersionPlatform(val version: String, val storeUrl: String)
