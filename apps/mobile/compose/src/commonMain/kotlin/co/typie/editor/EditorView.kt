package co.typie.editor

import androidx.compose.foundation.focusable
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.runtime.Composable
import androidx.compose.runtime.DisposableEffect
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.SideEffect
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.focus.focusRequester
import androidx.compose.ui.focus.onFocusChanged
import androidx.compose.ui.platform.LocalDensity
import androidx.compose.ui.platform.LocalFocusManager
import androidx.compose.ui.unit.dp
import co.typie.editor.body.EditorDocumentLayoutSpec
import co.typie.editor.body.resolvePaginatedPageGap
import co.typie.editor.external.EditorExternalElementOverlay
import co.typie.editor.ffi.Message
import co.typie.editor.ffi.SystemEvent
import co.typie.editor.ffi.Viewport
import co.typie.editor.input.editorInput
import co.typie.editor.interaction.editorInteractions
import co.typie.editor.overlay.EditorCursorOverlay
import co.typie.editor.overlay.EditorLineHighlightOverlay
import co.typie.editor.runtime.LocalEditorRuntime
import co.typie.editor.runtime.LocalEditorUiState
import co.typie.editor.scroll.LocalEditorBringIntoViewRequests
import co.typie.editor.surface.EditorPageSurface
import co.typie.editor.surface.editorPagePositionTracker
import co.typie.platform.PlatformModule
import co.typie.storage.Preference
import kotlinx.coroutines.CancellationException

@Composable
internal fun EditorView(
  graph: ByteArray,
  layoutSpec: EditorDocumentLayoutSpec,
  viewportWidth: Float,
  viewportHeight: Float,
  modifier: Modifier = Modifier,
  textInputSessionEnabled: Boolean = true,
  suppressSoftwareKeyboard: Boolean = false,
  showDebugSurfaceOverlay: Boolean = false,
) {
  val platform = PlatformModule.platform
  val density = LocalDensity.current
  val scope = rememberCoroutineScope()
  val runtime = LocalEditorRuntime.current
  val uiState = LocalEditorUiState.current
  val bringIntoViewRequests = LocalEditorBringIntoViewRequests.current
  val zoomController = LocalEditorZoomController.current
  val displayZoom = zoomController.displayZoom
  val themeVariant = currentEditorThemeVariant()
  val canCreateEditor = runtime.canCreateEditor

  LaunchedEffect(canCreateEditor, viewportWidth, viewportHeight, density.density, themeVariant) {
    if (viewportWidth <= 0f || viewportHeight <= 0f) {
      return@LaunchedEffect
    }
    if (!canCreateEditor) {
      return@LaunchedEffect
    }

    val scaleFactor = density.density.toDouble()
    val viewport =
      Viewport(width = viewportWidth, height = viewportHeight, scaleFactor = scaleFactor)
    if (runtime.editor == null) {
      uiState.clear()
      try {
        val editor =
          Editor.create(
            graph = graph,
            viewport = viewport,
            themeVariant = themeVariant,
            scope = scope,
            onError = { editor, error -> runtime.reportError(editor, error) },
          )
        runtime.attach(editor)
      } catch (e: CancellationException) {
        throw e
      } catch (e: Throwable) {
        runtime.reportError(e)
      }
    }
  }

  Box(modifier) {
    val editor = runtime.editor ?: return@Box
    val focusManager = LocalFocusManager.current
    LaunchedEffect(editor, themeVariant) {
      editor.enqueue(Message.System(SystemEvent.SetThemeVariant(themeVariant)))
    }
    SideEffect { editor.focusManager = focusManager }
    DisposableEffect(editor, uiState) {
      onDispose {
        uiState.clear()
        runtime.clear(editor)
      }
    }

    Box(
      Modifier.fillMaxWidth()
        .focusRequester(editor.focusRequester)
        .onFocusChanged {
          uiState.updateFocus(it.isFocused)
          editor.enqueue(Message.System(SystemEvent.SetFocused(it.isFocused)))
        }
        .editorInput(
          editor = editor,
          uiState = uiState,
          platform = platform,
          bringIntoViewRequests = bringIntoViewRequests,
          textInputSessionEnabled = textInputSessionEnabled,
          suppressSoftwareKeyboard = suppressSoftwareKeyboard,
        )
        .focusable()
        .editorInteractions(
          editor = editor,
          bringIntoViewRequests = bringIntoViewRequests,
          uiState = uiState,
          density = density.density,
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
          val pageCursor = editor.cursor?.takeIf { it.pageIdx == index }
          val pageExternalElements = editor.externalElements.filter { it.pageIdx == index }
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
            showDebugOverlay = showDebugSurfaceOverlay,
            backgroundOverlay = {
              EditorLineHighlightOverlay(
                cursor = pageCursor,
                focused = uiState.focused,
                displayZoom = displayZoom,
                pageWidth = size.width,
                enabled = Preference.lineHighlightEnabled,
              )
            },
            foregroundOverlay = {
              EditorExternalElementOverlay(
                elements = pageExternalElements,
                displayZoom = displayZoom,
              )
              EditorCursorOverlay(
                cursor = pageCursor,
                focused = uiState.focused,
                displayZoom = displayZoom,
              )
              // TODO(editor-parity): selection rect, composition rect, 인라인 맞춤법 하이라이트,
              // 인라인 리마크 하이라이트 같은 foreground overlay도 surface-local로 채워 넣어야 한다.
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
    }
  }
}
