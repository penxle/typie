package co.typie.screen.editor.editor.overlay

import androidx.compose.ui.Modifier
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.geometry.Rect
import co.typie.editor.interaction.EditorInteractionController
import co.typie.editor.interaction.EditorPointerCoordinateResolver
import co.typie.editor.interaction.editorInteractions

internal fun Modifier.editorOverlayInteractions(
  density: Float,
  interactionController: EditorInteractionController,
  editorRectInOverlay: Rect,
  touchTargetTopLeftInOverlay: Offset,
): Modifier =
  editorInteractions(
    density = density,
    interactionController = interactionController,
    coordinateResolver =
      EditorOverlayPointerCoordinateResolver(
        editorRectInOverlay = editorRectInOverlay,
        touchTargetTopLeftInOverlay = touchTargetTopLeftInOverlay,
      ),
  )

private class EditorOverlayPointerCoordinateResolver(
  private val editorRectInOverlay: Rect,
  private val touchTargetTopLeftInOverlay: Offset,
) : EditorPointerCoordinateResolver {
  override fun positionForPointerStart(position: Offset): Offset = positionInEditor(position)

  override fun positionForTapStart(position: Offset): Offset = positionInEditor(position)

  override fun positionForActivePointer(position: Offset): Offset = positionInEditor(position)

  private fun positionInEditor(position: Offset): Offset =
    touchTargetTopLeftInOverlay + position - editorRectInOverlay.topLeft
}
