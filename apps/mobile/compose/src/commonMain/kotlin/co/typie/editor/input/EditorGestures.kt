package co.typie.editor.input

import androidx.compose.ui.Modifier
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.input.pointer.PointerEvent
import androidx.compose.ui.input.pointer.PointerEventPass
import androidx.compose.ui.input.pointer.PointerId
import androidx.compose.ui.input.pointer.changedToUp
import androidx.compose.ui.node.ModifierNodeElement
import androidx.compose.ui.node.PointerInputModifierNode
import androidx.compose.ui.unit.IntSize
import co.typie.editor.Editor
import co.typie.editor.ffi.Message
import co.typie.editor.ffi.PointerEvent as EditorPointerEvent
import co.typie.editor.runtime.EditorUiState
import co.typie.editor.scroll.EditorBringIntoViewRequests
import co.typie.editor.scroll.EditorBringIntoViewTarget
import co.typie.editor.scroll.awaitWithBringIntoView
import kotlinx.coroutines.launch

private const val EditorTapSlopDp = 8f

internal fun Modifier.editorGestures(
  editor: Editor,
  bringIntoViewRequests: EditorBringIntoViewRequests,
  uiState: EditorUiState,
  density: Float,
): Modifier =
  this then
    EditorGesturesElement(
      editor = editor,
      bringIntoViewRequests = bringIntoViewRequests,
      uiState = uiState,
      density = density,
    )

private data class EditorGesturesElement(
  private val editor: Editor,
  private val bringIntoViewRequests: EditorBringIntoViewRequests,
  private val uiState: EditorUiState,
  private val density: Float,
) : ModifierNodeElement<EditorGesturesNode>() {
  override fun create(): EditorGesturesNode =
    EditorGesturesNode(
      editor = editor,
      bringIntoViewRequests = bringIntoViewRequests,
      uiState = uiState,
      density = density,
    )

  override fun update(node: EditorGesturesNode) {
    node.editor = editor
    node.bringIntoViewRequests = bringIntoViewRequests
    node.uiState = uiState
    node.density = density
  }
}

private class EditorGesturesNode(
  var editor: Editor,
  var bringIntoViewRequests: EditorBringIntoViewRequests,
  var uiState: EditorUiState,
  var density: Float,
) : Modifier.Node(), PointerInputModifierNode {
  private var activePointerId: PointerId? = null
  private var downPositionInNode = Offset.Zero
  private var movedPastTapSlop = false

  override fun onPointerEvent(pointerEvent: PointerEvent, pass: PointerEventPass, bounds: IntSize) {
    if (pass != PointerEventPass.Main) {
      return
    }

    if (density <= 0f) {
      return
    }

    val activePointerId = activePointerId
    val tapSlopPx = EditorTapSlopDp * density

    if (activePointerId == null) {
      val down = pointerEvent.changes.firstOrNull { it.pressed && !it.previousPressed } ?: return
      this.activePointerId = down.id
      downPositionInNode = down.position
      movedPastTapSlop = false
      return
    }

    val change = pointerEvent.changes.firstOrNull { it.id == activePointerId } ?: return
    if ((change.position - downPositionInNode).getDistance() > tapSlopPx) {
      movedPastTapSlop = true
    }

    if (!change.pressed) {
      if (change.changedToUp() && !movedPastTapSlop) {
        editor.focus()
        change.consume()
        val xDp = downPositionInNode.x / density
        val yDp = downPositionInNode.y / density
        val point =
          uiState
            .resolveViewportTransform(pageSizes = editor.pageSizes)
            .globalToLocal(x = xDp, y = yDp)
        if (point != null) {
          coroutineScope.launch {
            editor.awaitWithBringIntoView(bringIntoViewRequests) {
              enqueue(
                Message.Pointer(
                  EditorPointerEvent.Down(page = point.page, x = point.x, y = point.y, count = 1)
                )
              )
              beforeCommit { bringIntoView(EditorBringIntoViewTarget.CurrentCursorLine) }
            }
          }
        }
      }
      resetPointerState()
    }
  }

  override fun onCancelPointerInput() {
    resetPointerState()
  }

  private fun resetPointerState() {
    activePointerId = null
    downPositionInNode = Offset.Zero
    movedPastTapSlop = false
  }

  override fun onDetach() {
    resetPointerState()
    super.onDetach()
  }
}
