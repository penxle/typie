package co.typie.screen.editor.editor.overlay

import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.runtime.Composable
import androidx.compose.runtime.SideEffect
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
import co.typie.editor.ext.unclippedBoundsInRoot
import co.typie.editor.ffi.Selection
import co.typie.editor.ffi.SelectionExpansionUnit
import co.typie.editor.runtime.EditorContextMenuMode
import co.typie.editor.runtime.LocalEditorRuntime
import co.typie.editor.runtime.LocalEditorUiState
import co.typie.editor.scroll.EditorAutoScrollPolicy
import co.typie.editor.scroll.EditorVisibleArea
import co.typie.editor.scroll.LocalEditorBringIntoViewRequests
import co.typie.editor.scroll.resolveKeepVisibleRange
import co.typie.editor.viewport.EditorViewportState
import co.typie.screen.editor.editor.subpane.EditorTableAxisActionsTarget
import kotlin.math.roundToInt

@Composable
internal fun EditorScreenOverlayHost(
  viewportState: EditorViewportState,
  visibleArea: EditorVisibleArea,
  autoScrollPolicy: EditorAutoScrollPolicy,
  onTableAxisActionsRequest: (EditorTableAxisActionsTarget, Selection?) -> Unit,
  editorMutationEnabled: Boolean = true,
  showDebugOverlay: Boolean = false,
  modifier: Modifier = Modifier,
) {
  val density = LocalDensity.current
  val runtime = LocalEditorRuntime.current
  val uiState = LocalEditorUiState.current
  val editorRectInViewport: () -> Rect? = {
    uiState.editorBoundsInContainer
      .toPxRect(density.density)
      ?.translate(
        translateX = -viewportState.scrollOffset.x * density.density,
        translateY =
          visibleArea.headerHeight * density.density -
            (viewportState.scrollOffset.y * density.density).roundToInt(),
      )
  }
  Box(modifier = modifier.fillMaxSize()) {
    if (showDebugOverlay) {
      DebugViewportLine(y = visibleArea.visibleViewportTop, color = Color(0xFF00C853))
      DebugViewportLine(y = visibleArea.visibleViewportBottom, color = Color(0xFF00C853))

      if (autoScrollPolicy.typewriterActive) {
        autoScrollPolicy.targetTop?.let { DebugViewportLine(y = it, color = Color(0xFFFFAB00)) }
        autoScrollPolicy.targetBottom
          ?.takeIf { autoScrollPolicy.targetLineHeight > 0f }
          ?.let { DebugViewportLine(y = it, color = Color(0xFFFFAB00)) }
      } else {
        val keepVisibleRange = resolveKeepVisibleRange(visibleArea)
        DebugViewportLine(y = keepVisibleRange.top, color = Color(0xFFFFAB00))
        DebugViewportLine(y = keepVisibleRange.bottom, color = Color(0xFFFFAB00))
      }
    }

    val editor = runtime.editor
    if (editor != null && editorMutationEnabled) {
      EditorTableAxisSelectionOverlay(
        editor = editor,
        uiState = uiState,
        editorRectInOverlay = editorRectInViewport,
        density = density.density,
        onTableAxisActionsRequest = onTableAxisActionsRequest,
      )
    }
  }
}

@Composable
internal fun EditorContextMenuHost(
  visibleArea: EditorVisibleArea,
  onCommentRequest: (() -> Unit)?,
  editorMutationEnabled: Boolean,
  modifier: Modifier = Modifier,
) {
  val density = LocalDensity.current
  val runtime = LocalEditorRuntime.current
  val uiState = LocalEditorUiState.current
  val contextMenu = uiState.contextMenu
  val bringIntoViewRequests = LocalEditorBringIntoViewRequests.current
  var overlayBoundsInRoot by remember { mutableStateOf<Rect?>(null) }

  Box(
    modifier =
      modifier.fillMaxSize().onGloballyPositioned { coordinates ->
        overlayBoundsInRoot = coordinates.unclippedBoundsInRoot()
      }
  ) {
    val editor = runtime.editor ?: return@Box
    val overlayBounds = overlayBoundsInRoot ?: return@Box
    val editorRectInOverlay =
      uiState
        .editorRectInRoot()
        ?.translate(translateX = -overlayBounds.left, translateY = -overlayBounds.top) ?: return@Box
    var lastPresentation by
      remember(editor) { mutableStateOf<EditorContextMenuPresentation?>(null) }
    val presentation =
      if (contextMenu.isVisibleFor(editor.publishedState)) {
        val anchor =
          resolveContextMenuAnchor(
            editor = editor,
            uiState = uiState,
            editorRectInOverlay = editorRectInOverlay,
            density = density.density,
          )
        val availableExpansionUnits = rememberAvailableExpansionUnits(editor)
        if (anchor != null && availableExpansionUnits != null) {
          EditorContextMenuPresentation(
            mode = contextMenu.mode,
            anchor = anchor,
            visibleArea = visibleArea,
            editorMutationEnabled = editorMutationEnabled,
            actions =
              rememberEditorContextMenuActions(
                editor = editor,
                bringIntoViewRequests = bringIntoViewRequests,
                contextMenu = contextMenu,
                availableExpansionUnits = availableExpansionUnits,
                editorMutationEnabled = editorMutationEnabled,
                onComment = onCommentRequest,
              ),
          )
        } else null
      } else null
    SideEffect { if (presentation != null) lastPresentation = presentation }
    val displayed = presentation ?: lastPresentation ?: return@Box
    EditorSelectionContextMenuOverlay(
      visible = presentation != null,
      onHidden = { lastPresentation = null },
      mode = displayed.mode,
      onExpandMenu = contextMenu::expand,
      anchor = displayed.anchor,
      overlaySize = overlayBounds.size,
      visibleArea = displayed.visibleArea,
      actions = displayed.actions,
      editorMutationEnabled = displayed.editorMutationEnabled,
      onBoundsInWindowChanged = { contextMenu.boundsInWindow = it },
    )
  }
}

// Dismissal clears the selection anchor and mode immediately; retain only the final presentation
// until its exit animation finishes, without keeping the menu logically open.
private data class EditorContextMenuPresentation(
  val mode: EditorContextMenuMode,
  val anchor: EditorContextMenuAnchor,
  val visibleArea: EditorVisibleArea,
  val editorMutationEnabled: Boolean,
  val actions: EditorContextMenuActions,
)

@Composable
private fun rememberAvailableExpansionUnits(editor: Editor): Set<SelectionExpansionUnit>? {
  val expansion = editor.publishedState.blockState?.expansion ?: return null
  return remember(expansion) {
    buildSet {
      if (expansion.word) add(SelectionExpansionUnit.Word)
      if (expansion.sentence) add(SelectionExpansionUnit.Sentence)
      if (expansion.paragraph) add(SelectionExpansionUnit.Paragraph)
      if (expansion.all) add(SelectionExpansionUnit.All)
    }
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
