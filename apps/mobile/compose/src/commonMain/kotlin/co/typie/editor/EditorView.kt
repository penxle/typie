package co.typie.editor

import androidx.compose.foundation.focusable
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.runtime.Composable
import androidx.compose.runtime.DisposableEffect
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.focus.focusRequester
import androidx.compose.ui.focus.onFocusChanged
import androidx.compose.ui.platform.LocalDensity
import androidx.compose.ui.platform.LocalFocusManager
import androidx.compose.ui.unit.dp
import co.typie.editor.body.EditorDocumentLayoutSpec
import co.typie.editor.body.resolvePaginatedPageGap
import co.typie.editor.ffi.Doc
import co.typie.editor.ffi.Selection
import co.typie.editor.ffi.Viewport
import co.typie.editor.input.editorGestures
import co.typie.editor.input.editorInput
import co.typie.editor.overlay.EditorOverlayHost
import co.typie.editor.runtime.LocalEditorRuntime
import co.typie.editor.runtime.LocalEditorUiState
import co.typie.editor.scroll.LocalEditorAutoScrollController
import co.typie.editor.surface.EditorPageSurface
import co.typie.editor.surface.editorPagePositionTracker
import co.typie.platform.PlatformModule

@Composable
internal fun EditorView(
  doc: Doc,
  selection: Selection,
  layoutSpec: EditorDocumentLayoutSpec,
  viewportWidth: Float,
  viewportHeight: Float,
  modifier: Modifier = Modifier,
) {
  val platform = PlatformModule.platform
  val density = LocalDensity.current
  val scope = rememberCoroutineScope()
  val runtime = LocalEditorRuntime.current
  val uiState = LocalEditorUiState.current
  val zoomController = LocalEditorZoomController.current
  val autoScrollController = LocalEditorAutoScrollController.current
  var appliedDoc by remember { mutableStateOf<Doc?>(null) }
  var appliedSelection by remember { mutableStateOf<Selection?>(null) }
  val displayZoom = zoomController.displayZoom

  LaunchedEffect(doc, selection, viewportWidth, viewportHeight, density.density) {
    if (viewportWidth <= 0f || viewportHeight <= 0f) {
      return@LaunchedEffect
    }

    val scaleFactor = density.density.toDouble()
    val currentEditor = runtime.editor
    val shouldRecreate = currentEditor == null || appliedDoc != doc || appliedSelection != selection
    if (shouldRecreate) {
      uiState.clear()
      runtime.attach(
        Editor.create(
          doc,
          selection,
          Viewport(width = viewportWidth, height = viewportHeight, scaleFactor = scaleFactor),
          scope,
        )
      )
      appliedDoc = doc
      appliedSelection = selection
    } else {
      currentEditor.resizeViewport(
        width = viewportWidth,
        height = viewportHeight,
        scaleFactor = scaleFactor,
      )
      // TODO(editor-parity): 엔진이 incremental sync 훅을 노출하면, doc/selection 변경 시
      // 세션 재생성 대신 delta만 기존 에디터 세션에 적용해야 한다.
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
        .editorInput(editor, platform, autoScrollController)
        .focusable()
        .editorGestures(
          editor = editor,
          uiState = uiState,
          density = density.density,
          autoScrollController = autoScrollController,
        )
    ) {
      val pageSpacing =
        when (layoutSpec) {
          is EditorDocumentLayoutSpec.Continuous -> 0.dp
          is EditorDocumentLayoutSpec.Paginated -> resolvePaginatedPageGap(displayZoom).dp
        // TODO(editor-parity): 실제 paginated page gap과 paper chrome 감각은
        // Flutter/Web 기준으로 더 정교하게 맞춰야 한다.
        }
      val showPageChrome = layoutSpec is EditorDocumentLayoutSpec.Paginated
      Column(
        horizontalAlignment = Alignment.CenterHorizontally,
        verticalArrangement = Arrangement.spacedBy(pageSpacing),
      ) {
        editor.pageSizes.forEachIndexed { index, size ->
          EditorPageSurface(
            page = index,
            width = size.width,
            height = size.height,
            showChrome = showPageChrome,
            debugBottomMarginHeight =
              when (layoutSpec) {
                is EditorDocumentLayoutSpec.Paginated -> layoutSpec.pageMarginBottom
                is EditorDocumentLayoutSpec.Continuous -> 0f
              },
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
