package co.typie.dev

import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.setValue

enum class NetworkPreset {
  Normal,
  Slow,
  Offline,
}

class SimulatedNetworkFailureException : Exception("Simulated network failure")

object NetworkSimulator {
  var preset by mutableStateOf(NetworkPreset.Normal)
    private set

  fun select(preset: NetworkPreset) {
    this.preset = preset
  }
}
