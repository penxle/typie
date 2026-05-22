package co.typie.editor.gesture

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
import co.typie.editor.runtime.EditorUiState
import co.typie.editor.scroll.EditorBringIntoViewRequests
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

internal fun Modifier.editorGestures(
  editor: Editor,
  bringIntoViewRequests: EditorBringIntoViewRequests,
  uiState: EditorUiState,
  density: Float,
  coordinateResolver: EditorPointerCoordinateResolver = DirectEditorPointerCoordinateResolver,
): Modifier =
  this then
    EditorGesturesElement(
      editor = editor,
      bringIntoViewRequests = bringIntoViewRequests,
      uiState = uiState,
      density = density,
      coordinateResolver = coordinateResolver,
    )

private data class EditorGesturesElement(
  private val editor: Editor,
  private val bringIntoViewRequests: EditorBringIntoViewRequests,
  private val uiState: EditorUiState,
  private val density: Float,
  private val coordinateResolver: EditorPointerCoordinateResolver,
) : ModifierNodeElement<EditorGesturesNode>() {
  override fun create(): EditorGesturesNode =
    EditorGesturesNode(
      editor = editor,
      bringIntoViewRequests = bringIntoViewRequests,
      uiState = uiState,
      density = density,
      coordinateResolver = coordinateResolver,
    )

  override fun update(node: EditorGesturesNode) {
    node.editor = editor
    node.bringIntoViewRequests = bringIntoViewRequests
    node.uiState = uiState
    node.density = density
    node.coordinateResolver = coordinateResolver
  }
}

private class EditorGesturesNode(
  var editor: Editor,
  var bringIntoViewRequests: EditorBringIntoViewRequests,
  var uiState: EditorUiState,
  var density: Float,
  var coordinateResolver: EditorPointerCoordinateResolver,
) : Modifier.Node(), PointerInputModifierNode {
  private val interactionSession = EditorInteractionSession(tapSlopPx = 0f)
  private var tapDispatchJob: Job? = null

  override fun onPointerEvent(pointerEvent: PointerEvent, pass: PointerEventPass, bounds: IntSize) {
    if (pass != PointerEventPass.Main) {
      return
    }

    if (density <= 0f) {
      cancelInteraction(editor)
      return
    }

    val tapSlopPx = EditorTapSlopDp * density
    interactionSession.updateTapSlop(tapSlopPx)

    pointerEvent.changes
      .filter { it.pressed && !it.previousPressed }
      .forEach { change ->
        val position = coordinateResolver.positionForStart(change.position) ?: return@forEach

        val result =
          interactionSession.onPointerDown(
            pointerId = change.id.value,
            position = position,
            nowMillis = change.uptimeMillis,
          )
        applyPointerResult(editor = editor, result = result)
        if (!interactionSession.isIgnoringUntilAllPointersUp) {
          scheduleTapDispatchJob(
            dispatchAtMillis = change.uptimeMillis + EditorTapDispatchDelayMillis
          )
        }
      }

    pointerEvent.changes
      .filter { it.pressed && it.previousPressed }
      .forEach { change ->
        val position = activePositionOrCancel(change.position) ?: return@forEach
        val result =
          interactionSession.onPointerMove(
            pointerId = change.id.value,
            position = position,
            nowMillis = change.uptimeMillis,
          )
        applyPointerResult(editor = editor, result = result)
      }

    pointerEvent.changes
      .filter { it.changedToUp() }
      .forEach { change ->
        val position = activePositionOrCancel(change.position) ?: return@forEach
        val result =
          interactionSession.onPointerUp(
            pointerId = change.id.value,
            position = position,
            nowMillis = change.uptimeMillis,
          )
        applyPointerResult(editor = editor, result = result)
        if (result.consume) {
          change.consume()
        }
      }
  }

  override fun onCancelPointerInput() {
    cancelInteraction(editor)
  }

  private fun scheduleTapDispatchJob(dispatchAtMillis: Long) {
    tapDispatchJob?.cancel()
    tapDispatchJob = coroutineScope.launch {
      try {
        delay(EditorTapDispatchDelayMillis)
        interactionSession
          .onTapTimer(
            nowMillis = dispatchAtMillis,
            isSelectionHit = ::isSelectionHit,
            hasRangeSelection = { editor.hasRangeSelection() },
          )
          ?.let { tap -> dispatchTap(editor = editor, tap = tap) }
      } finally {
        tapDispatchJob = null
      }
    }
  }

  private fun isSelectionHit(position: Offset): Boolean {
    val point = resolvePoint(positionInNode = position) ?: return true
    return point.page < 0 || editor.isSelectionHit(point)
  }

  private fun applyPointerResult(editor: Editor, result: EditorInteractionPointerResult) {
    if (result.cancelTapDispatch) {
      tapDispatchJob?.cancel()
      tapDispatchJob = null
    }
    if (result.cancelPointerStream) {
      editor.enqueue(Message.Pointer(EditorPointerEvent.Cancel))
    }
    result.tapDispatch?.let {
      tapDispatchJob?.cancel()
      tapDispatchJob = null
      dispatchTap(editor = editor, tap = it)
    }
  }

  private fun dispatchTap(editor: Editor, tap: EditorInteractionTapDispatch) {
    val point = resolvePoint(positionInNode = tap.position) ?: return
    if (!interactionSession.canDispatchTap(page = point.page)) {
      return
    }
    val previousCursor = editor.cursor
    editor.focus()
    coroutineScope.launch {
      editor.dispatchPrimaryTap(
        bringIntoViewRequests = bringIntoViewRequests,
        point = point,
        clickCount = tap.clickCount,
        previousCursor = previousCursor,
      )
    }
  }

  private fun resolvePoint(positionInNode: Offset): PagePoint? {
    val xDp = positionInNode.x / density
    val yDp = positionInNode.y / density
    return uiState
      .resolveViewportTransform(pageSizes = editor.pageSizes)
      .globalToLocal(x = xDp, y = yDp)
  }

  private fun cancelInteraction(editor: Editor) {
    applyPointerResult(editor = editor, result = interactionSession.cancel())
  }

  private fun activePositionOrCancel(position: Offset): Offset? {
    val mapped = coordinateResolver.positionForActivePointer(position)
    if (mapped == null && interactionSession.hasActivePointer) {
      cancelInteraction(editor)
    }
    return mapped
  }

  override fun onDetach() {
    cancelInteraction(editor)
    interactionSession.reset()
    super.onDetach()
  }
}
