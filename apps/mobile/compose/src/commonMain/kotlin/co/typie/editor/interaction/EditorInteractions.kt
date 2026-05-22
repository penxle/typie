package co.typie.editor.interaction

import androidx.compose.ui.Modifier
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.input.pointer.PointerEvent
import androidx.compose.ui.input.pointer.PointerEventPass
import androidx.compose.ui.input.pointer.changedToUp
import androidx.compose.ui.node.ModifierNodeElement
import androidx.compose.ui.node.PointerInputModifierNode
import androidx.compose.ui.unit.IntSize
import co.typie.editor.Editor
import co.typie.editor.PagePoint
import co.typie.editor.ffi.Message
import co.typie.editor.ffi.PointerEvent as EditorPointerEvent
import co.typie.editor.interaction.gestures.EditorTapDispatchDelayMillis
import co.typie.editor.runtime.EditorUiState
import co.typie.editor.scroll.EditorBringIntoViewRequests
import co.typie.editor.scroll.EditorBringIntoViewTarget
import kotlinx.coroutines.Job
import kotlinx.coroutines.delay
import kotlinx.coroutines.launch

private const val EditorTapSlopDp = 8f

internal interface EditorPointerCoordinateResolver {
  fun positionForStart(position: Offset): Offset?

  fun positionForActivePointer(position: Offset): Offset?
}

private data object DirectEditorPointerCoordinateResolver : EditorPointerCoordinateResolver {
  override fun positionForStart(position: Offset): Offset = position

  override fun positionForActivePointer(position: Offset): Offset = position
}

internal fun Modifier.editorInteractions(
  editor: Editor,
  bringIntoViewRequests: EditorBringIntoViewRequests,
  uiState: EditorUiState,
  density: Float,
  coordinateResolver: EditorPointerCoordinateResolver = DirectEditorPointerCoordinateResolver,
): Modifier =
  this then
    EditorInteractionsElement(
      editor = editor,
      bringIntoViewRequests = bringIntoViewRequests,
      uiState = uiState,
      density = density,
      coordinateResolver = coordinateResolver,
    )

private data class EditorInteractionsElement(
  private val editor: Editor,
  private val bringIntoViewRequests: EditorBringIntoViewRequests,
  private val uiState: EditorUiState,
  private val density: Float,
  private val coordinateResolver: EditorPointerCoordinateResolver,
) : ModifierNodeElement<EditorInteractionsNode>() {
  override fun create(): EditorInteractionsNode =
    EditorInteractionsNode(
      editor = editor,
      bringIntoViewRequests = bringIntoViewRequests,
      uiState = uiState,
      density = density,
      coordinateResolver = coordinateResolver,
    )

  override fun update(node: EditorInteractionsNode) {
    node.editor = editor
    node.bringIntoViewRequests = bringIntoViewRequests
    node.uiState = uiState
    node.density = density
    node.coordinateResolver = coordinateResolver
  }
}

private class EditorInteractionsNode(
  var editor: Editor,
  var bringIntoViewRequests: EditorBringIntoViewRequests,
  var uiState: EditorUiState,
  var density: Float,
  var coordinateResolver: EditorPointerCoordinateResolver,
) : Modifier.Node(), PointerInputModifierNode, EditorInteractionControllerHost {
  private val interactionController =
    EditorInteractionController(editorProvider = { editor }, host = this)
  private var tapDispatchJob: Job? = null

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
        val position = coordinateResolver.positionForStart(change.position) ?: return@forEach
        if (
          interactionController.onPointerDown(
            pointerId = change.id.value,
            position = position,
            nowMillis = change.uptimeMillis,
          )
        ) {
          change.consume()
        }
      }

    pointerEvent.changes
      .filter { it.pressed && it.previousPressed }
      .forEach { change ->
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
        val position = activePositionOrCancel(change.position) ?: return@forEach
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
  }

  override fun onCancelPointerInput() {
    cancelInteraction()
  }

  override fun resolvePoint(positionInNode: Offset): PagePoint? {
    val xDp = positionInNode.x / density
    val yDp = positionInNode.y / density
    return uiState
      .resolveViewportTransform(pageSizes = editor.pageSizes)
      .globalToLocal(x = xDp, y = yDp)
  }

  override fun scheduleTapDispatch(dispatchAtMillis: Long) {
    tapDispatchJob?.cancel()
    tapDispatchJob = coroutineScope.launch {
      try {
        delay(EditorTapDispatchDelayMillis)
        interactionController.onTapTimer(nowMillis = dispatchAtMillis)
      } finally {
        tapDispatchJob = null
      }
    }
  }

  override fun cancelTapDispatch() {
    tapDispatchJob?.cancel()
    tapDispatchJob = null
  }

  override fun launchInteraction(block: suspend () -> Unit) {
    coroutineScope.launch { block() }
  }

  override fun requestFocus(editor: Editor): Boolean = editor.focus()

  override fun enqueuePointerCancel() {
    editor.enqueue(Message.Pointer(EditorPointerEvent.Cancel))
  }

  override fun requestCurrentCursorLine(version: Long) {
    bringIntoViewRequests.requestForVersion(
      target = EditorBringIntoViewTarget.CurrentCursorLine,
      version = version,
    )
  }

  private fun cancelInteraction() {
    interactionController.cancel()
  }

  private fun activePositionOrCancel(position: Offset): Offset? {
    val mapped = coordinateResolver.positionForActivePointer(position)
    if (mapped == null && interactionController.hasActivePointer) {
      cancelInteraction()
    }
    return mapped
  }

  override fun onDetach() {
    cancelInteraction()
    interactionController.reset()
    super.onDetach()
  }
}
