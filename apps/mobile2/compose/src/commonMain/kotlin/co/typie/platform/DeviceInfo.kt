package co.typie.platform

data class DeviceInfoSnapshot(
  val platform: String,
  val osVersion: String,
  val appVersion: String,
  val deviceName: String? = null,
)

interface DeviceInfo {
  suspend fun snapshot(): DeviceInfoSnapshot
}
