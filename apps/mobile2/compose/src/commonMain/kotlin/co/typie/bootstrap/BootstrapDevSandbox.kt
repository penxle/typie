package co.typie.bootstrap

import co.typie.platform.Platform
import co.typie.platform.PlatformModule
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow

private const val BOOTSTRAP_DEV_STORE_URL = "https://apps.apple.com/app/id6745595771"

enum class BootstrapDevScenario(
  val label: String,
) {
  RemoteData("실제 상태 사용"),
  Ready("정상"),
  Maintenance("점검 중"),
  UpdateRequired("업데이트 필요"),
}

object BootstrapDevSandbox {
  private val _scenario = MutableStateFlow(BootstrapDevScenario.RemoteData)
  val scenario: StateFlow<BootstrapDevScenario> = _scenario

  val enabled: Boolean
    get() = PlatformModule.platform == Platform.Desktop

  val usesSandbox: Boolean
    get() = enabled && _scenario.value != BootstrapDevScenario.RemoteData

  val currentState: BootstrapState?
    get() = bootstrapDevState(_scenario.value)

  fun select(next: BootstrapDevScenario) {
    if (!enabled) return
    _scenario.value = next
  }
}

fun bootstrapDevState(scenario: BootstrapDevScenario): BootstrapState? {
  return when (scenario) {
    BootstrapDevScenario.RemoteData -> null
    BootstrapDevScenario.Ready -> BootstrapState.Ready
    BootstrapDevScenario.Maintenance -> BootstrapState.Maintenance(
      title = "점검 중",
      message = "데스크톱 DevTools에서 강제로 표시한 상태입니다.",
      until = null,
    )

    BootstrapDevScenario.UpdateRequired -> BootstrapState.UpdateRequired(
      storeUrl = BOOTSTRAP_DEV_STORE_URL,
      currentVersion = "1.0.0",
      requiredVersion = "9.9.9",
    )
  }
}

fun effectiveBootstrapState(
  remoteState: BootstrapState,
  scenario: BootstrapDevScenario,
): BootstrapState {
  return bootstrapDevState(scenario) ?: remoteState
}
