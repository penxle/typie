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
import kotlin.math.abs

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

  internal suspend fun settle(velocityX: Float, velocityThresholdPx: Float) {
    val target = releaseTarget(velocityX, velocityThresholdPx)
    runProgrammatic { state.animateTo(target, DrawerDefaults.AnimationSpec) }
  }

  internal fun releaseTarget(velocity: Float, velocityThreshold: Float): DrawerAnchor {
    // Mirrors Foundation's anchored-draggable target selection with Material3's drawer threshold.
    val offset = state.requireOffset()
    val anchors = state.anchors
    if (velocity == 0f) return anchors.closestAnchor(offset)!!

    val movingForward = velocity > 0f
    if (abs(velocity) >= velocityThreshold) {
      return anchors.closestAnchor(offset, searchUpwards = movingForward)!!
    }

    val lowerAnchor = anchors.closestAnchor(offset, searchUpwards = false)!!
    val upperAnchor = anchors.closestAnchor(offset, searchUpwards = true)!!
    val origin = if (movingForward) lowerAnchor else upperAnchor
    val destination = if (movingForward) upperAnchor else lowerAnchor
    val distance = abs(anchors.positionOf(upperAnchor) - anchors.positionOf(lowerAnchor))
    val threshold = distance * DrawerDefaults.PositionalThresholdFraction
    return if (abs(offset - anchors.positionOf(origin)) >= threshold) destination else origin
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
