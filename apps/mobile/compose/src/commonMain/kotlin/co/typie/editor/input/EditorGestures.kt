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
import co.typie.editor.scroll.EditorAutoScrollController
import co.typie.editor.scroll.EditorScrollTarget
import kotlinx.coroutines.launch

private const val EditorTapSlopDp = 8f

internal fun Modifier.editorGestures(
  editor: Editor,
  uiState: EditorUiState,
  density: Float,
  autoScrollController: EditorAutoScrollController?,
): Modifier =
  this then
    EditorGesturesElement(
      editor = editor,
      uiState = uiState,
      density = density,
      autoScrollController = autoScrollController,
    )

private data class EditorGesturesElement(
  private val editor: Editor,
  private val uiState: EditorUiState,
  private val density: Float,
  private val autoScrollController: EditorAutoScrollController?,
) : ModifierNodeElement<EditorGesturesNode>() {
  override fun create(): EditorGesturesNode =
    EditorGesturesNode(
      editor = editor,
      uiState = uiState,
      density = density,
      autoScrollController = autoScrollController,
    )

  override fun update(node: EditorGesturesNode) {
    node.editor = editor
    node.uiState = uiState
    node.density = density
    node.autoScrollController = autoScrollController
  }
}

private class EditorGesturesNode(
  var editor: Editor,
  var uiState: EditorUiState,
  var density: Float,
  var autoScrollController: EditorAutoScrollController?,
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
            editor.dispatch(
              Message.Pointer(
                EditorPointerEvent.Down(page = point.page, x = point.x, y = point.y, count = 1)
              )
            )
            autoScrollController?.request(target = EditorScrollTarget.CurrentCursor)
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
    autoScrollController = null
    super.onDetach()
  }
}
