package co.typie.ui.component.tooltip

import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.setValue
import androidx.compose.ui.geometry.Rect
import androidx.compose.ui.layout.LayoutCoordinates
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Job
import kotlinx.coroutines.delay
import kotlinx.coroutines.launch

enum class TooltipPlacement {
  Above,
  Below,
}

internal class TooltipAnchor {
  var coordinates: LayoutCoordinates? = null
  var placement by mutableStateOf(TooltipPlacement.Below)
  var text by mutableStateOf("")
  var shortcut by mutableStateOf<String?>(null)
  var bounds by mutableStateOf(Rect.Zero)
}

/** One hover delay and active anchor for the window, including transfers between controls. */
internal class TooltipState(private val scope: CoroutineScope) {
  var active by mutableStateOf<TooltipAnchor?>(null)
    private set

  private var pending: TooltipAnchor? = null
  private var showJob: Job? = null
  private var leaveJob: Job? = null
  private var warmJob: Job? = null
  private var warm = false

  fun enter(anchor: TooltipAnchor) {
    leaveJob?.cancel()
    warmJob?.cancel()
    showJob?.cancel()
    if (warm) {
      active = anchor
      pending = null
    } else {
      pending = anchor
      showJob = scope.launch {
        delay(500)
        pending = null
        active = anchor
        warm = true
      }
    }
  }

  fun leave(anchor: TooltipAnchor) {
    if (pending === anchor) {
      showJob?.cancel()
      pending = null
    }
    if (active !== anchor) return
    leaveJob?.cancel()
    leaveJob = scope.launch {
      delay(80)
      active = null
    }
  }

  fun remove(anchor: TooltipAnchor) {
    if (active === anchor || pending === anchor) dismiss()
  }

  /** Ordinary target clicks close the bubble but preserve the shared hover delay. */
  fun close(anchor: TooltipAnchor) {
    if (active !== anchor && pending !== anchor) return
    clear()
  }

  private fun clear() {
    showJob?.cancel()
    leaveJob?.cancel()
    pending = null
    active = null
  }

  /** Called by the host after its actual outro, including reduced-motion completion. */
  fun onHidden() {
    if (active != null || !warm) return
    warmJob?.cancel()
    warmJob = scope.launch {
      delay(300)
      warm = false
    }
  }

  /** Navigation, modals and window deactivation start a fresh hover session. */
  fun dismiss() {
    clear()
    warmJob?.cancel()
    warm = false
  }
}
