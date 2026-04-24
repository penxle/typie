package co.typie.editor.body

import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.BoxScope
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.runtime.Composable
import androidx.compose.ui.Modifier
import androidx.compose.ui.geometry.Offset
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
import co.typie.editor.scroll.EditorAutoScrollController
import co.typie.editor.scroll.EditorScrollTarget
import co.typie.editor.scroll.LocalEditorAutoScrollController
import kotlinx.coroutines.launch

private const val ExtensionTapSlopDp = 8f

@Composable
internal fun EditorExtensionArea(
  forwardingEnabled: Boolean,
  modifier: Modifier = Modifier,
  content: @Composable BoxScope.() -> Unit,
) {
  val density = LocalDensity.current
  val runtime = LocalEditorRuntime.current
  val uiState = LocalEditorUiState.current
  val autoScrollController = LocalEditorAutoScrollController.current
  val extensionAreaModifier =
    if (forwardingEnabled) {
      Modifier.editorExtensionForwarding(
        runtime = runtime,
        uiState = uiState,
        density = density.density,
        autoScrollController = autoScrollController,
      )
    } else {
      Modifier
    }

  Box(modifier = modifier.fillMaxWidth().then(extensionAreaModifier), content = content)
}

private fun Modifier.editorExtensionForwarding(
  runtime: EditorRuntime,
  uiState: EditorUiState,
  density: Float,
  autoScrollController: EditorAutoScrollController?,
): Modifier =
  this then
    EditorExtensionForwardingElement(
      runtime = runtime,
      uiState = uiState,
      density = density,
      autoScrollController = autoScrollController,
    )

private data class EditorExtensionForwardingElement(
  private val runtime: EditorRuntime,
  private val uiState: EditorUiState,
  private val density: Float,
  private val autoScrollController: EditorAutoScrollController?,
) : ModifierNodeElement<EditorExtensionForwardingNode>() {
  override fun create(): EditorExtensionForwardingNode =
    EditorExtensionForwardingNode(
      runtime = runtime,
      uiState = uiState,
      density = density,
      autoScrollController = autoScrollController,
    )

  override fun update(node: EditorExtensionForwardingNode) {
    node.runtime = runtime
    node.uiState = uiState
    node.density = density
    node.autoScrollController = autoScrollController
  }
}

private class EditorExtensionForwardingNode(
  var runtime: EditorRuntime,
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
        runtime.focus()
        val globalX = xDp - editorBounds.x
        val globalY = yDp - editorBounds.y
        val point =
          uiState
            .resolveViewportTransform(pageSizes = editor.pageSizes)
            .globalToLocal(x = globalX, y = globalY) ?: return resetPointerState()
        coroutineScope.launch {
          editor.dispatch(
            Message.Pointer(
              EditorPointerEvent.Down(page = point.page, x = point.x, y = point.y, count = 1)
            ),
            Message.Pointer(EditorPointerEvent.Up),
          )
          autoScrollController?.request(target = EditorScrollTarget.CurrentCursorLine)
        }
        // TODO(editor-parity): Compose 상호작용 런타임이 웹/플러터 수준으로 맞춰지면,
        // extension area에서도 down/move/up 전체 제스처 시퀀스를 포워딩해야 한다.
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

  override fun onDetach() {
    autoScrollController = null
    super.onDetach()
  }
}
