package co.typie.screen.editor.editor.overlay

import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.geometry.Rect
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.graphicsLayer
import androidx.compose.ui.layout.onGloballyPositioned
import androidx.compose.ui.platform.LocalDensity
import androidx.compose.ui.unit.dp
import co.typie.editor.Editor
import co.typie.editor.body.EditorDocumentLayoutSpec
import co.typie.editor.ext.unclippedBoundsInRoot
import co.typie.editor.ffi.Message
import co.typie.editor.ffi.Selection
import co.typie.editor.ffi.SelectionExpansionUnit
import co.typie.editor.ffi.SelectionOp
import co.typie.editor.ffi.Size as PageSize
import co.typie.editor.runtime.LocalEditorRuntime
import co.typie.editor.runtime.LocalEditorUiState
import co.typie.editor.scroll.EditorAutoScrollMode
import co.typie.editor.scroll.EditorAutoScrollPolicy
import co.typie.editor.scroll.EditorVisibleArea
import co.typie.editor.scroll.LocalEditorBringIntoViewRequests
import co.typie.editor.viewport.EditorViewportState
import co.typie.screen.editor.editor.subpane.EditorTableAxisActionsTarget
import kotlin.math.roundToInt

@Composable
internal fun EditorScreenOverlayHost(
  viewportState: EditorViewportState,
  visibleArea: EditorVisibleArea,
  autoScrollPolicy: EditorAutoScrollPolicy,
  layoutSpec: EditorDocumentLayoutSpec,
  pageSizes: List<PageSize>,
  displayZoom: Float,
  onTableAxisActionsRequest: (EditorTableAxisActionsTarget, Selection?) -> Unit,
  showDebugOverlay: Boolean = false,
  modifier: Modifier = Modifier,
) {
  val density = LocalDensity.current
  val runtime = LocalEditorRuntime.current
  val uiState = LocalEditorUiState.current
  val contextMenu = uiState.contextMenu
  val bringIntoViewRequests = LocalEditorBringIntoViewRequests.current
  var overlayBoundsInRoot by remember { mutableStateOf<Rect?>(null) }
  val overlayBounds = overlayBoundsInRoot
  val editorBoundsInContainer = uiState.editorBoundsInContainer
  val editorRectInViewport =
    if (editorBoundsInContainer.isValid && density.density > 0f) {
      val left =
        editorBoundsInContainer.x * density.density - viewportState.scrollOffset.x * density.density
      val top =
        (visibleArea.headerHeight + editorBoundsInContainer.y) * density.density -
          (viewportState.scrollOffset.y * density.density).roundToInt()
      Rect(
        left = left,
        top = top,
        right = left + editorBoundsInContainer.width * density.density,
        bottom = top + editorBoundsInContainer.height * density.density,
      )
    } else {
      null
    }
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
      if (editor != null) {
        if (editorRectInViewport != null) {
          EditorTableColumnResizeOverlay(
            editor = editor,
            uiState = uiState,
            editorRectInOverlay = editorRectInViewport,
            density = density.density,
          )
          EditorTableCellSelectionOverlay(
            editor = editor,
            uiState = uiState,
            editorRectInOverlay = editorRectInViewport,
            density = density.density,
          )
          EditorTableAxisSelectionOverlay(
            editor = editor,
            uiState = uiState,
            editorRectInOverlay = editorRectInViewport,
            overlaySize = overlayBounds.size,
            density = density.density,
            onTableAxisActionsRequest = onTableAxisActionsRequest,
          )
          EditorSelectionHandleOverlay(
            editor = editor,
            uiState = uiState,
            editorRectInOverlay = editorRectInViewport,
            density = density.density,
          )
        }

        if (editorRectInOverlay != null && contextMenu.isVisibleFor(editor.state)) {
          val anchor =
            resolveContextMenuAnchor(
              editor = editor,
              uiState = uiState,
              editorRectInOverlay = editorRectInOverlay,
              density = density.density,
            )
          if (anchor != null) {
            val availableExpansionUnits = rememberAvailableExpansionUnits(editor)
            if (availableExpansionUnits != null) {
              val actions =
                rememberEditorContextMenuActions(
                  editor = editor,
                  bringIntoViewRequests = bringIntoViewRequests,
                  contextMenu = contextMenu,
                  availableExpansionUnits = availableExpansionUnits,
                )

              EditorSelectionContextMenuOverlay(
                anchor = anchor,
                overlaySize = overlayBounds.size,
                visibleArea = visibleArea,
                showCopyCutActions = actions.showCopyCutActions,
                availableExpansionUnits = actions.availableExpansionUnits,
                onCopy = actions.onCopy,
                onCut = actions.onCut,
                onPaste = actions.onPaste,
                onExpandWord = actions.onExpandWord,
                onExpandSentence = actions.onExpandSentence,
                onExpandParagraph = actions.onExpandParagraph,
                onSelectAll = actions.onSelectAll,
                onDismiss = actions.onDismiss,
              )
            }
          }
        }
      }
    }
  }
}

@Composable
private fun rememberAvailableExpansionUnits(editor: Editor): Set<SelectionExpansionUnit>? {
  var units by
    remember(editor, editor.state.selection) { mutableStateOf<Set<SelectionExpansionUnit>?>(null) }
  LaunchedEffect(editor, editor.state.version) {
    units =
      SelectionExpansionUnit.entries
        .filter { unit -> editor.can(Message.Selection(SelectionOp.Expand(unit))) }
        .toSet()
  }
  return units
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
