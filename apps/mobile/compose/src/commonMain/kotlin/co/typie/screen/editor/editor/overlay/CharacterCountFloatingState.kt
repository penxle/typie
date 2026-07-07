package co.typie.screen.editor.editor.overlay

import androidx.compose.runtime.Stable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableFloatStateOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.setValue

/**
 * Holds the position and expand/collapse state of the character-count floating widget.
 *
 * Position is stored as a relative fraction (0..1) of the free space (viewport minus widget size)
 * so it survives viewport resizes and rotations, mirroring the legacy widget's behavior. Absolute
 * pixel offsets are derived once the viewport is measured.
 *
 * Kept free of `@Composable` so the drag/clamp/persist behavior is unit-testable.
 */
@Stable
class CharacterCountFloatingState(
  private var relativeX: Float,
  private var relativeY: Float,
  private val persist: (relativeX: Float, relativeY: Float) -> Unit,
) {
  var offsetX by mutableFloatStateOf(0f)
    private set

  var offsetY by mutableFloatStateOf(0f)
    private set

  var expanded by mutableStateOf(false)
    private set

  private var freeWidth = 0f
  private var freeHeight = 0f

  // Top of the draggable area: the header / top safe area the widget must not cover.
  private var minY = 0f

  private var dragging = false

  fun onViewportMeasured(
    width: Float,
    height: Float,
    widgetWidth: Float,
    widgetHeight: Float,
    topOcclusion: Float = 0f,
    bottomOcclusion: Float = 0f,
  ) {
    freeWidth = (width - widgetWidth).coerceAtLeast(0f)
    freeHeight = (height - topOcclusion - bottomOcclusion - widgetHeight).coerceAtLeast(0f)
    minY = topOcclusion

    if (dragging) {
      // The viewport changed mid-drag (e.g. the keyboard opened): keep the dragged position and
      // only clamp it into the new bounds instead of resetting from the stale relative fraction.
      offsetX = offsetX.coerceIn(0f, freeWidth)
      offsetY = offsetY.coerceIn(minY, minY + freeHeight)
      return
    }

    offsetX = (relativeX * freeWidth).coerceIn(0f, freeWidth)
    offsetY = (minY + relativeY * freeHeight).coerceIn(minY, minY + freeHeight)
  }

  fun onDragStart() {
    dragging = true
  }

  fun onDrag(dx: Float, dy: Float) {
    offsetX = (offsetX + dx).coerceIn(0f, freeWidth)
    offsetY = (offsetY + dy).coerceIn(minY, minY + freeHeight)
  }

  fun onDragEnd() {
    dragging = false
    relativeX = if (freeWidth > 0f) offsetX / freeWidth else 0f
    relativeY = if (freeHeight > 0f) (offsetY - minY) / freeHeight else 0f
    persist(relativeX, relativeY)
  }

  fun toggleExpanded() {
    expanded = !expanded
  }
}
