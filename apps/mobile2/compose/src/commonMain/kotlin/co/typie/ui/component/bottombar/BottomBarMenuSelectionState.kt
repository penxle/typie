package co.typie.ui.component.bottombar

import androidx.compose.runtime.Composable
import androidx.compose.runtime.Stable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateMapOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.geometry.Rect
import co.typie.ui.component.popover.PressGestureSession

@Stable
internal class BottomBarMenuSelectionState {
  private val itemBounds = mutableStateMapOf<Int, Rect>()
  private var trackedPointerInWindow by mutableStateOf<Offset?>(null)
  private var suppressedClickIndex by mutableStateOf<Int?>(null)

  var activeIndex by mutableStateOf<Int?>(null)
    private set

  fun prepareGesture() {
    suppressedClickIndex = null
  }

  fun updateItemBounds(index: Int, bounds: Rect) {
    itemBounds[index] = bounds
    recomputeActiveIndex()
  }

  fun consumeSuppressedClick(index: Int): Boolean {
    if (suppressedClickIndex != index) {
      return false
    }

    suppressedClickIndex = null
    return true
  }

  fun syncSession(session: PressGestureSession?): Int? {
    if (session == null) {
      clearPointer()
      return null
    }

    if (!session.isArmed) {
      if (session.isReleased) {
        clearPointer()
      }
      return null
    }

    trackedPointerInWindow = session.positionInWindow
    recomputeActiveIndex()

    if (!session.isReleased) {
      return null
    }

    val selectedIndex = activeIndex?.takeIf { it == hitTest(session.positionInWindow) }
    if (selectedIndex != null) {
      suppressedClickIndex = selectedIndex
    }
    clearPointer()
    return selectedIndex
  }

  fun clearPointer() {
    trackedPointerInWindow = null
    activeIndex = null
  }

  fun reset() {
    itemBounds.clear()
    clearPointer()
    suppressedClickIndex = null
  }

  private fun recomputeActiveIndex() {
    activeIndex = trackedPointerInWindow?.let(::hitTest)
  }

  private fun hitTest(positionInWindow: Offset): Int? {
    for ((index, bounds) in itemBounds) {
      if (bounds.contains(positionInWindow)) {
        return index
      }
    }
    return null
  }
}

@Composable
internal fun rememberBottomBarMenuSelectionState(): BottomBarMenuSelectionState {
  return remember { BottomBarMenuSelectionState() }
}
