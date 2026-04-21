package co.typie.screen.editor.editor.state

import androidx.compose.foundation.ScrollState
import androidx.compose.runtime.Stable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableFloatStateOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.setValue
import androidx.compose.runtime.withFrameNanos
import co.typie.editor.body.EditorDocumentLayoutSpec
import co.typie.editor.ffi.Size
import co.typie.editor.runtime.EditorRuntime
import co.typie.editor.runtime.EditorUiState
import co.typie.screen.editor.editor.layout.EditorBodyGeometry
import co.typie.screen.editor.editor.layout.EditorMeasuredSize
import co.typie.screen.editor.editor.layout.EditorVisibleArea
import co.typie.screen.editor.editor.layout.resolveEditorBodyGeometry

@Stable
internal class EditorScreenState internal constructor(val scrollState: ScrollState) {
  var viewport by mutableStateOf(EditorMeasuredSize())
    private set

  var sceneInForeground by mutableStateOf(true)
    private set

  var headerHeight by mutableFloatStateOf(0f)
    private set

  var toolbarHeight by mutableFloatStateOf(0f)
    private set

  fun updateViewport(size: EditorMeasuredSize) {
    if (viewport == size) {
      return
    }

    viewport = size
  }

  fun updateHeaderHeight(height: Float) {
    if (headerHeight == height) {
      return
    }

    headerHeight = height
  }

  fun updateToolbarHeight(height: Float) {
    if (toolbarHeight == height) {
      return
    }

    toolbarHeight = height
  }

  fun updateSceneForeground(isForeground: Boolean, runtime: EditorRuntime, uiState: EditorUiState) {
    if (sceneInForeground == isForeground) {
      return
    }

    sceneInForeground = isForeground
    if (!isForeground) {
      uiState.updateFocus(false)
      runtime.deactivateScene()
      updateToolbarHeight(0f)
    }
  }

  suspend fun prepareToLeaveEditorScene(
    runtime: EditorRuntime,
    uiState: EditorUiState,
    flushDrafts: suspend () -> Unit,
  ) {
    uiState.updateFocus(false)
    runtime.deactivateScene()
    flushDrafts()
    withFrameNanos {}
  }

  fun shouldShowToolbar(bodyFocused: Boolean): Boolean = sceneInForeground && bodyFocused

  fun resolveVisibleArea(topInset: Float, rawImeInset: Float): EditorVisibleArea =
    EditorVisibleArea(
      viewport = viewport,
      headerHeight = headerHeight,
      topInset = topInset,
      imeInset = if (sceneInForeground) rawImeInset else 0f,
      toolbarHeight = toolbarHeight,
    )

  fun resolveBodyGeometry(
    topInset: Float,
    rawImeInset: Float,
    layoutSpec: EditorDocumentLayoutSpec,
    pageSizes: List<Size>,
  ): EditorBodyGeometry =
    resolveEditorBodyGeometry(
      visibleArea = resolveVisibleArea(topInset = topInset, rawImeInset = rawImeInset),
      layoutSpec = layoutSpec,
      pageSizes = pageSizes,
    )
}
