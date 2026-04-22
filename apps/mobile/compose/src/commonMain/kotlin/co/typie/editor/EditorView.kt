package co.typie.editor

import androidx.compose.foundation.focusable
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.runtime.Composable
import androidx.compose.runtime.DisposableEffect
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.ui.Modifier
import androidx.compose.ui.focus.focusRequester
import androidx.compose.ui.focus.onFocusChanged
import androidx.compose.ui.platform.LocalDensity
import androidx.compose.ui.platform.LocalFocusManager
import co.typie.editor.ffi.Doc
import co.typie.editor.ffi.Selection
import co.typie.editor.ffi.Viewport
import co.typie.editor.input.editorGestures
import co.typie.editor.input.editorInput
import co.typie.editor.overlay.EditorOverlayHost
import co.typie.editor.runtime.LocalEditorRuntime
import co.typie.editor.runtime.LocalEditorUiState
import co.typie.editor.surface.EditorPageSurface
import co.typie.editor.surface.editorPagePositionTracker
import co.typie.platform.PlatformModule
import co.typie.screen.editor.editor.scroll.LocalEditorScrollController

@Composable
internal fun EditorView(
  doc: Doc,
  selection: Selection,
  viewportWidth: Float,
  viewportHeight: Float,
  modifier: Modifier = Modifier,
) {
  val platform = PlatformModule.platform
  val density = LocalDensity.current
  val scope = rememberCoroutineScope()
  val runtime = LocalEditorRuntime.current
  val uiState = LocalEditorUiState.current
  val scrollController = LocalEditorScrollController.current

  LaunchedEffect(doc, selection, viewportWidth, viewportHeight, density.density) {
    if (viewportWidth <= 0f || viewportHeight <= 0f) {
      return@LaunchedEffect
    }

    val scaleFactor = density.density.toDouble()
    val currentEditor = runtime.editor
    if (currentEditor == null) {
      uiState.clear()
      runtime.attach(
        Editor.create(
          doc,
          selection,
          Viewport(width = viewportWidth, height = viewportHeight, scaleFactor = scaleFactor),
          scope,
        )
      )
    } else {
      currentEditor.resizeViewport(
        width = viewportWidth,
        height = viewportHeight,
        scaleFactor = scaleFactor,
      )
      // TODO(editor-parity): Apply document and selection deltas to the live editor session
      // once the engine exposes incremental sync hooks instead of recreate-or-ignore behavior.
    }
  }

  Box(modifier) {
    val editor = runtime.editor ?: return@Box
    editor.focusManager = LocalFocusManager.current
    DisposableEffect(editor, uiState) {
      onDispose {
        uiState.clear()
        runtime.clear(editor)
      }
    }

    Box(
      Modifier.fillMaxWidth()
        .focusRequester(editor.focusRequester)
        .onFocusChanged { uiState.updateFocus(it.isFocused) }
        .editorInput(editor, platform, scrollController)
        .focusable()
        .editorGestures(
          editor = editor,
          uiState = uiState,
          density = density.density,
          scrollController = scrollController,
        )
    ) {
      Column {
        editor.pageSizes.forEachIndexed { index, size ->
          EditorPageSurface(
            page = index,
            width = size.width,
            height = size.height,
            modifier =
              Modifier.editorPagePositionTracker(
                uiState = uiState,
                page = index,
                density = density.density,
              ),
          )
        }
      }

      EditorOverlayHost()
    }
  }
}
