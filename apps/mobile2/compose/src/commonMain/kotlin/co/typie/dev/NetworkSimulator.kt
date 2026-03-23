package co.typie.dev

import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import org.koin.core.annotation.Single

enum class NetworkPreset {
  Normal,
  Slow,
  Offline,
}

@Single
class NetworkSimulator {
  private val _preset = MutableStateFlow(NetworkPreset.Normal)
  val preset: StateFlow<NetworkPreset> = _preset

  fun select(preset: NetworkPreset) {
    _preset.value = preset
  }
}
