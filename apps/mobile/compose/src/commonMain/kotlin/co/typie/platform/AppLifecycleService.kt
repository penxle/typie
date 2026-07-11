package co.typie.platform

import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow

internal enum class AppLifecycleState {
  Foreground,
  Background,
}

internal data class AppLifecycleSnapshot(
  val state: AppLifecycleState,
  val foregroundGeneration: Long,
)

internal class AppLifecycleService {
  private val mutableSnapshot =
    MutableStateFlow(
      AppLifecycleSnapshot(state = AppLifecycleState.Background, foregroundGeneration = 0L)
    )
  private var hasBeenForeground = false

  val snapshot: StateFlow<AppLifecycleSnapshot> = mutableSnapshot.asStateFlow()

  fun update(foreground: Boolean) {
    val current = mutableSnapshot.value
    val nextState = if (foreground) AppLifecycleState.Foreground else AppLifecycleState.Background
    if (current.state == nextState) return

    val returnedToForeground = nextState == AppLifecycleState.Foreground && hasBeenForeground
    mutableSnapshot.value =
      AppLifecycleSnapshot(
        state = nextState,
        foregroundGeneration = current.foregroundGeneration + if (returnedToForeground) 1 else 0,
      )
    if (nextState == AppLifecycleState.Foreground) {
      hasBeenForeground = true
    }
  }
}

internal val appLifecycleService = AppLifecycleService()
