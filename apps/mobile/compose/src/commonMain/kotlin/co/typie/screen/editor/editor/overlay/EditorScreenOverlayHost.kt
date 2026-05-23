package co.typie.screen.editor.editor.overlay

import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.geometry.Rect
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.graphicsLayer
import androidx.compose.ui.layout.LayoutCoordinates
import androidx.compose.ui.layout.onGloballyPositioned
import androidx.compose.ui.layout.positionInRoot
import androidx.compose.ui.platform.LocalDensity
import androidx.compose.ui.unit.dp
import co.typie.editor.body.EditorDocumentLayoutSpec
import co.typie.editor.ffi.Size as PageSize
import co.typie.editor.interaction.LocalEditorInteractionScope
import co.typie.editor.runtime.LocalEditorRuntime
import co.typie.editor.runtime.LocalEditorUiState
import co.typie.editor.scroll.EditorAutoScrollMode
import co.typie.editor.scroll.EditorAutoScrollPolicy
import co.typie.editor.scroll.EditorVisibleArea
import co.typie.editor.viewport.EditorViewportState

@Composable
internal fun EditorScreenOverlayHost(
  viewportState: EditorViewportState,
  visibleArea: EditorVisibleArea,
  autoScrollPolicy: EditorAutoScrollPolicy,
  layoutSpec: EditorDocumentLayoutSpec,
  pageSizes: List<PageSize>,
  displayZoom: Float,
  showDebugOverlay: Boolean = false,
  modifier: Modifier = Modifier,
) {
  val density = LocalDensity.current
  val interactionController = LocalEditorInteractionScope.current.controller
  val runtime = LocalEditorRuntime.current
  val uiState = LocalEditorUiState.current
  var overlayBoundsInRoot by remember { mutableStateOf<Rect?>(null) }
  val overlayBounds = overlayBoundsInRoot
  val editorRectInOverlay = overlayBounds?.let { bounds ->
    uiState.editorRectInRoot()?.translate(translateX = -bounds.left, translateY = -bounds.top)
  }

  Box(
    modifier =
      modifier.fillMaxSize().onGloballyPositioned { coordinates ->
        overlayBoundsInRoot = coordinates.unclippedBoundsInRoot()
      }
  ) {
    EditorScrollbars(
      viewportState = viewportState,
      visibleArea = visibleArea,
      layoutSpec = layoutSpec,
      pageSizes = pageSizes,
      displayZoom = displayZoom,
      modifier = Modifier.fillMaxSize(),
    )

    if (showDebugOverlay) {
      DebugViewportLine(y = visibleArea.visibleViewportTop, color = Color(0xFF00C853))
      DebugViewportLine(y = visibleArea.visibleViewportBottom, color = Color(0xFF00C853))

      when (autoScrollPolicy.mode) {
        EditorAutoScrollMode.KeepCursorVisible -> {
          DebugViewportLine(y = autoScrollPolicy.keepVisibleRange.top, color = Color(0xFFFFAB00))
          DebugViewportLine(y = autoScrollPolicy.keepVisibleRange.bottom, color = Color(0xFFFFAB00))
        }

        EditorAutoScrollMode.Typewriter -> {
          autoScrollPolicy.targetTop?.let { DebugViewportLine(y = it, color = Color(0xFFFFAB00)) }
          autoScrollPolicy.targetBottom
            ?.takeIf { autoScrollPolicy.targetLineHeight > 0f }
            ?.let { DebugViewportLine(y = it, color = Color(0xFFFFAB00)) }
        }
      }
    }

    if (overlayBounds != null) {
      val editor = runtime.editor
      if (
        editor != null &&
          editorRectInOverlay != null &&
          interactionController.isContextMenuVisibleFor(editor.state)
      ) {
        val anchor =
          resolveContextMenuAnchor(
            editor = editor,
            uiState = uiState,
            editorRectInOverlay = editorRectInOverlay,
            density = density.density,
          )
        if (anchor != null) {
          EditorSelectionContextMenuOverlay(
            anchor = anchor,
            overlaySize = overlayBounds.size,
            visibleArea = visibleArea,
          )
        }
      }
    }

    // TODO(editor-parity): selection handle과 drag auto-scroll affordance를 포팅해야 한다.
  }
}

@Composable
private fun DebugViewportLine(y: Float, color: Color) {
  Box(
    modifier =
      Modifier.fillMaxWidth()
        .height(2.dp)
        .graphicsLayer { translationY = y.dp.toPx() }
        .background(color.copy(alpha = 0.9f))
  )
}

private fun LayoutCoordinates.unclippedBoundsInRoot(): Rect {
  val position = positionInRoot()
  return Rect(
    left = position.x,
    top = position.y,
    right = position.x + size.width,
    bottom = position.y + size.height,
  )
}
