package co.typie.platform

data class DeviceInfoData(
  val model: String,
  val osName: String,
  val osVersion: String,
  val appVersion: String,
  val appBuildNumber: String,
)

interface DeviceInfo {
  fun retrieve(): DeviceInfoData
}
