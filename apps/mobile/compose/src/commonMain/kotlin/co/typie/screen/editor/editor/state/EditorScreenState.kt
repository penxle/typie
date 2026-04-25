package co.typie.screen.editor.editor.state

import androidx.compose.runtime.Stable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableFloatStateOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.setValue
import androidx.compose.runtime.withFrameNanos
import androidx.compose.ui.geometry.Size
import co.typie.editor.runtime.EditorRuntime
import co.typie.editor.runtime.EditorUiState
import co.typie.editor.scroll.EditorVisibleArea
import co.typie.editor.viewport.EditorViewportState

@Stable
internal class EditorScreenState internal constructor(val viewportState: EditorViewportState) {
  val viewport: Size
    get() = viewportState.viewportSize

  var sceneInForeground by mutableStateOf(true)
    private set

  var headerHeight by mutableFloatStateOf(0f)
    private set

  fun updateHeaderHeight(height: Float) {
    if (headerHeight == height) {
      return
    }

    headerHeight = height
  }

  fun updateSceneForeground(isForeground: Boolean, runtime: EditorRuntime, uiState: EditorUiState) {
    if (sceneInForeground == isForeground) {
      return
    }

    sceneInForeground = isForeground
    if (!isForeground) {
      uiState.updateFocus(false)
      runtime.deactivateScene()
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

  fun resolveVisibleArea(
    topInset: Float,
    rawBottomSafeInset: Float,
    rawImeInset: Float,
  ): EditorVisibleArea =
    EditorVisibleArea(
      viewport = viewport,
      headerHeight = headerHeight,
      topInset = topInset,
      safeBottomInset = if (sceneInForeground) rawBottomSafeInset else 0f,
      imeInset = if (sceneInForeground) rawImeInset else 0f,
    )
}
