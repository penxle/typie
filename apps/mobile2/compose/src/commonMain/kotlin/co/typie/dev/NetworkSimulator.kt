package co.typie.dev

import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow

enum class NetworkPreset {
  Normal,
  Slow,
  Offline,
}

class SimulatedNetworkFailureException : Exception("Simulated network failure")

object NetworkSimulator {
  private val _preset = MutableStateFlow(NetworkPreset.Normal)
  val preset: StateFlow<NetworkPreset> = _preset

  fun select(preset: NetworkPreset) {
    _preset.value = preset
  }
}
