package co.typie.editor.surface

import androidx.compose.ui.Modifier
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.layout.LayoutCoordinates
import androidx.compose.ui.layout.positionInParent
import androidx.compose.ui.layout.positionInRoot
import androidx.compose.ui.node.GlobalPositionAwareModifierNode
import androidx.compose.ui.node.ModifierNodeElement
import co.typie.editor.runtime.EditorUiState

internal fun Modifier.editorPagePositionTracker(
  uiState: EditorUiState,
  page: Int,
  density: Float,
): Modifier =
  this then EditorPagePositionTrackerElement(uiState = uiState, page = page, density = density)

private data class EditorPagePositionTrackerElement(
  private val uiState: EditorUiState,
  private val page: Int,
  private val density: Float,
) : ModifierNodeElement<EditorPagePositionTrackerNode>() {
  override fun create(): EditorPagePositionTrackerNode =
    EditorPagePositionTrackerNode(uiState = uiState, page = page, density = density)

  override fun update(node: EditorPagePositionTrackerNode) {
    node.uiState = uiState
    node.page = page
    node.density = density
  }
}

private class EditorPagePositionTrackerNode(
  var uiState: EditorUiState,
  var page: Int,
  var density: Float,
) : Modifier.Node(), GlobalPositionAwareModifierNode {
  override fun onGloballyPositioned(coordinates: LayoutCoordinates) {
    if (density <= 0f) {
      return
    }

    val pos = coordinates.positionInParent()
    uiState.updatePageOffset(page = page, offset = Offset(pos.x / density, pos.y / density))
    uiState.updatePagePositionInRoot(
      page = page,
      positionInRoot = coordinates.positionInRoot(),
      density = density,
    )
  }

  override fun onDetach() {
    uiState.clearPageOffset(page)
    super.onDetach()
  }
}
