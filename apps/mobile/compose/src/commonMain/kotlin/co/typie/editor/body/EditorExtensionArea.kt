package co.typie.editor.body

import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.BoxScope
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.runtime.Composable
import androidx.compose.ui.Modifier
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.input.pointer.PointerEvent
import androidx.compose.ui.input.pointer.PointerEventPass
import androidx.compose.ui.input.pointer.PointerId
import androidx.compose.ui.input.pointer.changedToUp
import androidx.compose.ui.node.ModifierNodeElement
import androidx.compose.ui.node.PointerInputModifierNode
import androidx.compose.ui.platform.LocalDensity
import androidx.compose.ui.unit.IntSize
import co.typie.editor.ffi.Message
import co.typie.editor.ffi.PointerEvent as EditorPointerEvent
import co.typie.editor.runtime.EditorBoundsInContainer
import co.typie.editor.runtime.EditorRuntime
import co.typie.editor.runtime.EditorUiState
import co.typie.editor.runtime.LocalEditorRuntime
import co.typie.editor.runtime.LocalEditorUiState

private val DebugExtensionAreaColor = Color(0x2200D97A)
private const val ExtensionTapSlopDp = 8f

@Composable
internal fun EditorExtensionArea(
  modifier: Modifier = Modifier,
  content: @Composable BoxScope.() -> Unit,
) {
  val density = LocalDensity.current
  val runtime = LocalEditorRuntime.current
  val uiState = LocalEditorUiState.current

  Box(
    modifier =
      modifier
        .fillMaxWidth()
        .background(DebugExtensionAreaColor)
        .editorExtensionForwarding(runtime = runtime, uiState = uiState, density = density.density),
    content = content,
  )
}

private fun Modifier.editorExtensionForwarding(
  runtime: EditorRuntime,
  uiState: EditorUiState,
  density: Float,
): Modifier =
  this then
    EditorExtensionForwardingElement(runtime = runtime, uiState = uiState, density = density)

private data class EditorExtensionForwardingElement(
  private val runtime: EditorRuntime,
  private val uiState: EditorUiState,
  private val density: Float,
) : ModifierNodeElement<EditorExtensionForwardingNode>() {
  override fun create(): EditorExtensionForwardingNode =
    EditorExtensionForwardingNode(runtime = runtime, uiState = uiState, density = density)

  override fun update(node: EditorExtensionForwardingNode) {
    node.runtime = runtime
    node.uiState = uiState
    node.density = density
  }
}

private class EditorExtensionForwardingNode(
  var runtime: EditorRuntime,
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

    val editor = runtime.editor ?: return
    val editorBounds = uiState.editorBoundsInContainer
    if (!editorBounds.isValid || density <= 0f) {
      return
    }

    val tapSlopPx = ExtensionTapSlopDp * density
    val activePointerId = activePointerId

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
        val xDp = downPositionInNode.x / density
        val yDp = downPositionInNode.y / density
        if (isInsideEditorBounds(bounds = editorBounds, x = xDp, y = yDp)) {
          resetPointerState()
          return
        }

        change.consume()
        val globalX = xDp - editorBounds.x
        val globalY = yDp - editorBounds.y
        val point =
          uiState.globalToLocal(x = globalX, y = globalY, pageSizes = editor.pageSizes)
            ?: return resetPointerState()

        runtime.focus()
        editor.enqueue(
          Message.Pointer(
            EditorPointerEvent.Down(page = point.page, x = point.x, y = point.y, count = 1)
          )
        )
        editor.enqueue(Message.Pointer(EditorPointerEvent.Up))
        // TODO(editor-parity): Forward full down/move/up gesture sequences from extension areas
        // once the Compose interaction runtime matches web/flutter behavior.
      }
      resetPointerState()
    }
  }

  override fun onCancelPointerInput() {
    resetPointerState()
  }

  private fun isInsideEditorBounds(bounds: EditorBoundsInContainer, x: Float, y: Float): Boolean {
    return x >= bounds.x &&
      x <= bounds.x + bounds.width &&
      y >= bounds.y &&
      y <= bounds.y + bounds.height
  }

  private fun resetPointerState() {
    activePointerId = null
    downPositionInNode = Offset.Zero
    movedPastTapSlop = false
  }
}
