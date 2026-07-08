package co.typie.editor.surface

import androidx.compose.ui.Modifier
import androidx.compose.ui.layout.LayoutCoordinates
import androidx.compose.ui.layout.findRootCoordinates
import androidx.compose.ui.layout.positionInRoot
import androidx.compose.ui.node.GlobalPositionAwareModifierNode
import androidx.compose.ui.node.ModifierNodeElement

// Mirrors the web editor's IntersectionObserver rootMargin of '100% 0px': one viewport
// height of overscan above and below, vertical axis only so horizontal panning at high
// zoom never detaches the pages sharing the visible column.
private const val OverscanFactor = 1f

internal fun isEditorPageRenderActive(
  pageTop: Float,
  pageBottom: Float,
  rootHeight: Float,
): Boolean {
  if (rootHeight <= 0f) {
    return true
  }
  val overscan = rootHeight * OverscanFactor
  return pageBottom > -overscan && pageTop < rootHeight + overscan
}

internal fun Modifier.editorPageRenderGate(onActiveChange: (Boolean) -> Unit): Modifier =
  this then EditorPageRenderGateElement(onActiveChange)

private data class EditorPageRenderGateElement(private val onActiveChange: (Boolean) -> Unit) :
  ModifierNodeElement<EditorPageRenderGateNode>() {
  override fun create(): EditorPageRenderGateNode = EditorPageRenderGateNode(onActiveChange)

  override fun update(node: EditorPageRenderGateNode) {
    node.onActiveChange = onActiveChange
  }
}

private class EditorPageRenderGateNode(var onActiveChange: (Boolean) -> Unit) :
  Modifier.Node(), GlobalPositionAwareModifierNode {
  private var lastActive: Boolean? = null

  override fun onGloballyPositioned(coordinates: LayoutCoordinates) {
    val top = coordinates.positionInRoot().y
    val active =
      isEditorPageRenderActive(
        pageTop = top,
        pageBottom = top + coordinates.size.height,
        rootHeight = coordinates.findRootCoordinates().size.height.toFloat(),
      )
    if (active != lastActive) {
      lastActive = active
      onActiveChange(active)
    }
  }
}
