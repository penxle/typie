package co.typie.editor.interaction

import androidx.compose.ui.Modifier
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.input.pointer.PointerEvent
import androidx.compose.ui.input.pointer.PointerEventPass
import androidx.compose.ui.input.pointer.changedToUp
import androidx.compose.ui.input.pointer.isAltPressed
import androidx.compose.ui.input.pointer.isCtrlPressed
import androidx.compose.ui.input.pointer.isMetaPressed
import androidx.compose.ui.input.pointer.isShiftPressed
import androidx.compose.ui.node.ModifierNodeElement
import androidx.compose.ui.node.PointerInputModifierNode
import androidx.compose.ui.unit.IntSize
import co.typie.editor.ffi.InputModifiers

private const val EditorTapSlopDp = 8f

internal data class EditorPinchSample(val focalInRootPx: Offset, val distancePx: Float)

internal interface EditorPointerCoordinateResolver {
  fun positionInRoot(position: Offset): Offset?

  fun positionForPointerStart(position: Offset): Offset?

  fun positionForTapStart(position: Offset): Offset?

  fun positionForActivePointer(position: Offset): Offset?
}

internal fun Modifier.editorInteractions(
  density: Float,
  interactionController: EditorInteractionController,
  coordinateResolver: EditorPointerCoordinateResolver,
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
  private var ignorePinchPointersUntilAllUp = false

  override fun onPointerEvent(pointerEvent: PointerEvent, pass: PointerEventPass, bounds: IntSize) {
    if (pass != PointerEventPass.Main) {
      return
    }

    if (density <= 0f) {
      cancelInteractionIfActive()
      return
    }

    interactionController.updateTapSlop(tapSlopPx = EditorTapSlopDp * density)

    val pressedChanges = pointerEvent.changes.filter { it.pressed }
    if (ignorePinchPointersUntilAllUp) {
      if (pressedChanges.isEmpty()) {
        ignorePinchPointersUntilAllUp = false
      }
      pointerEvent.changes.forEach { it.consume() }
      return
    }
    if (interactionController.isPinching && pressedChanges.size != 2) {
      if (pressedChanges.size > 2) {
        interactionController.cancel()
      } else {
        interactionController.onPinchEnd()
      }
      pointerOwnership.reset()
      ignorePinchPointersUntilAllUp = pressedChanges.isNotEmpty()
      pointerEvent.changes.forEach { it.consume() }
      return
    }
    if (pressedChanges.size == 2) {
      val positionsInRoot = pressedChanges.mapNotNull { change ->
        coordinateResolver.positionInRoot(change.position)
      }
      val sample = resolveEditorPinchSample(positionsInRoot)
      if (sample != null && interactionController.onPinchSample(sample)) {
        pointerOwnership.reset()
        pointerEvent.changes.forEach { it.consume() }
        return
      }
    }

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
            inputModifiers = pointerEvent.inputModifiers(),
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
    cancelInteractionIfActive()
  }

  private fun cancelInteraction() {
    pointerOwnership.reset()
    ignorePinchPointersUntilAllUp = false
    interactionController.cancel()
  }

  private fun cancelInteractionIfActive() {
    if (
      pointerOwnership.hasPointers ||
        interactionController.isPinching ||
        ignorePinchPointersUntilAllUp
    ) {
      cancelInteraction()
    }
  }

  private fun activePositionOrCancel(position: Offset): Offset? {
    val mapped = coordinateResolver.positionForActivePointer(position)
    if (mapped == null) {
      cancelInteraction()
    }
    return mapped
  }

  override fun onDetach() {
    cancelInteractionIfActive()
    super.onDetach()
  }
}

internal fun resolveEditorPinchSample(positionsInRoot: List<Offset>): EditorPinchSample? {
  if (positionsInRoot.size != 2) {
    return null
  }
  val first = positionsInRoot[0]
  val second = positionsInRoot[1]
  return EditorPinchSample(
    focalInRootPx = (first + second) / 2f,
    distancePx = (first - second).getDistance(),
  )
}

private fun PointerEvent.inputModifiers(): InputModifiers {
  val modifiers = keyboardModifiers
  return InputModifiers(
    shift = modifiers.isShiftPressed,
    ctrl = modifiers.isCtrlPressed,
    alt = modifiers.isAltPressed,
    meta = modifiers.isMetaPressed,
  )
}
