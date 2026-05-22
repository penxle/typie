package co.typie.editor.interaction

import androidx.compose.ui.Modifier
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.input.pointer.PointerEvent
import androidx.compose.ui.input.pointer.PointerEventPass
import androidx.compose.ui.input.pointer.changedToUp
import androidx.compose.ui.node.ModifierNodeElement
import androidx.compose.ui.node.PointerInputModifierNode
import androidx.compose.ui.unit.IntSize

private const val EditorTapSlopDp = 8f

internal interface EditorPointerCoordinateResolver {
  fun positionForPointerStart(position: Offset): Offset?

  fun positionForTapStart(position: Offset): Offset?

  fun positionForActivePointer(position: Offset): Offset?
}

private data object DirectEditorPointerCoordinateResolver : EditorPointerCoordinateResolver {
  override fun positionForPointerStart(position: Offset): Offset = position

  override fun positionForTapStart(position: Offset): Offset = position

  override fun positionForActivePointer(position: Offset): Offset = position
}

internal fun Modifier.editorInteractions(
  density: Float,
  interactionController: EditorInteractionController,
  coordinateResolver: EditorPointerCoordinateResolver = DirectEditorPointerCoordinateResolver,
): Modifier =
  this then
    EditorInteractionsElement(
      density = density,
      coordinateResolver = coordinateResolver,
      interactionController = interactionController,
    )

private data class EditorInteractionsElement(
  private val density: Float,
  private val coordinateResolver: EditorPointerCoordinateResolver,
  private val interactionController: EditorInteractionController,
) : ModifierNodeElement<EditorInteractionsNode>() {
  override fun create(): EditorInteractionsNode =
    EditorInteractionsNode(
      density = density,
      coordinateResolver = coordinateResolver,
      interactionController = interactionController,
    )

  override fun update(node: EditorInteractionsNode) {
    node.density = density
    node.coordinateResolver = coordinateResolver
    node.interactionController = interactionController
  }
}

private class EditorInteractionsNode(
  var density: Float,
  var coordinateResolver: EditorPointerCoordinateResolver,
  var interactionController: EditorInteractionController,
) : Modifier.Node(), PointerInputModifierNode {
  private val pointerOwnership = EditorPointerOwnership()

  override fun onPointerEvent(pointerEvent: PointerEvent, pass: PointerEventPass, bounds: IntSize) {
    if (pass != PointerEventPass.Main) {
      return
    }

    if (density <= 0f) {
      cancelInteraction()
      return
    }

    interactionController.updateTapSlop(tapSlopPx = EditorTapSlopDp * density)

    pointerEvent.changes
      .filter { it.pressed && !it.previousPressed }
      .forEach { change ->
        val pointerPosition =
          coordinateResolver.positionForPointerStart(change.position) ?: return@forEach
        pointerOwnership.acquire(pointerId = change.id.value)
        val tapPosition = coordinateResolver.positionForTapStart(change.position)
        if (
          interactionController.onPointerDown(
            pointerId = change.id.value,
            position = tapPosition ?: pointerPosition,
            nowMillis = change.uptimeMillis,
            tapEnabled = tapPosition != null,
          )
        ) {
          change.consume()
        }
      }

    pointerEvent.changes
      .filter { it.pressed && it.previousPressed }
      .forEach { change ->
        if (!pointerOwnership.owns(pointerId = change.id.value)) {
          return@forEach
        }
        val position = activePositionOrCancel(change.position) ?: return@forEach
        if (
          interactionController.onPointerMove(
            pointerId = change.id.value,
            position = position,
            nowMillis = change.uptimeMillis,
          )
        ) {
          change.consume()
        }
      }

    pointerEvent.changes
      .filter { it.changedToUp() }
      .forEach { change ->
        if (!pointerOwnership.owns(pointerId = change.id.value)) {
          return@forEach
        }
        val position = activePositionOrCancel(change.position)
        if (position != null) {
          if (
            interactionController.onPointerUp(
              pointerId = change.id.value,
              position = position,
              nowMillis = change.uptimeMillis,
            )
          ) {
            change.consume()
          }
        }
        pointerOwnership.release(pointerId = change.id.value)
      }
  }

  override fun onCancelPointerInput() {
    cancelInteraction()
  }

  private fun cancelInteraction() {
    pointerOwnership.reset()
    interactionController.cancel()
  }

  private fun activePositionOrCancel(position: Offset): Offset? {
    val mapped = coordinateResolver.positionForActivePointer(position)
    if (mapped == null) {
      cancelInteraction()
    }
    return mapped
  }

  override fun onDetach() {
    cancelInteraction()
    super.onDetach()
  }
}
