package co.typie.ui.component.drawer

import androidx.compose.foundation.gestures.AnchoredDraggableState
import androidx.compose.foundation.gestures.DraggableAnchors
import androidx.compose.foundation.gestures.animateTo
import androidx.compose.runtime.Stable
import androidx.compose.runtime.compositionLocalOf
import androidx.compose.runtime.derivedStateOf
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.setValue

enum class DrawerAnchor {
  Closed,
  Open,
}

@Stable
class Drawer {
  internal val state: AnchoredDraggableState<DrawerAnchor> =
    AnchoredDraggableState(
      initialValue = DrawerAnchor.Closed,
      anchors =
        DraggableAnchors {
          DrawerAnchor.Closed at 0f
          DrawerAnchor.Open at 0f
        },
    )

  // Counter instead of a Boolean: when two programmatic close() calls overlap
  // (e.g. action dismiss + scrim tap), MutatorMutex cancels the older animateTo
  // and its finally block runs before the newer animateTo finishes. A Boolean
  // would flip back to false here even though another programmatic animation is
  // still in flight.
  private var programmaticAnimationCount by mutableStateOf(0)

  internal val isProgrammaticAnimating: Boolean
    get() = programmaticAnimationCount > 0

  val isOpen: Boolean by derivedStateOf { state.currentValue == DrawerAnchor.Open }

  suspend fun open() {
    runProgrammatic { state.animateTo(DrawerAnchor.Open, DrawerDefaults.AnimationSpec) }
  }

  suspend fun close() {
    runProgrammatic { state.animateTo(DrawerAnchor.Closed, DrawerDefaults.AnimationSpec) }
  }

  suspend fun toggle() {
    if (isOpen) close() else open()
  }

  suspend fun settle() {
    runProgrammatic { state.animateTo(state.targetValue, DrawerDefaults.AnimationSpec) }
  }

  private suspend inline fun runProgrammatic(block: () -> Unit) {
    programmaticAnimationCount++
    try {
      block()
    } finally {
      programmaticAnimationCount--
    }
  }
}

val LocalDrawer = compositionLocalOf<Drawer> { error("No Drawer provided") }
