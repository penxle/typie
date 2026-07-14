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
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberUpdatedState
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
import co.typie.editor.external.EditorExternalElementOverlay
import co.typie.editor.ffi.Message
import co.typie.editor.ffi.SystemEvent
import co.typie.editor.ffi.ThemeVariant
import co.typie.editor.ffi.Viewport
import co.typie.editor.input.editorInput
import co.typie.editor.overlay.EditorCursorOverlay
import co.typie.editor.overlay.EditorLineHighlightOverlay
import co.typie.editor.runtime.LocalEditorRuntime
import co.typie.editor.runtime.LocalEditorUiState
import co.typie.editor.scroll.LocalEditorBringIntoViewRequests
import co.typie.editor.surface.EditorPageSurface
import co.typie.editor.surface.editorPagePositionTracker
import co.typie.editor.sync.DocumentEditorLoad
import co.typie.platform.PlatformModule
import co.typie.storage.Preference
import kotlinx.coroutines.CancellationException

@Composable
internal fun EditorView(
  load: DocumentEditorLoad,
  layoutSpec: EditorDocumentLayoutSpec,
  viewportWidth: Float,
  viewportHeight: Float,
  modifier: Modifier = Modifier,
  editorInputEnabled: Boolean = true,
  suppressSoftwareKeyboard: Boolean = false,
  showDebugSurfaceOverlay: Boolean = false,
) {
  val platform = PlatformModule.platform
  val density = LocalDensity.current
  val runtime = LocalEditorRuntime.current
  val uiState = LocalEditorUiState.current
  val bringIntoViewRequests = LocalEditorBringIntoViewRequests.current
  val zoomController = LocalEditorZoomController.current
  val displayZoom = zoomController.displayZoom
  val themeVariant = currentEditorThemeVariant()
  val canCreateEditor = runtime.canCreateEditor
  val environment =
    EditorAttachEnvironment(
      width = viewportWidth,
      height = viewportHeight,
      scaleFactor = density.density.toDouble(),
      themeVariant = themeVariant,
    )
  val currentLoad by rememberUpdatedState(load)
  val currentEnvironment by rememberUpdatedState(environment)
  var editorThemeVariant by remember(load) { mutableStateOf<ThemeVariant?>(null) }

  LaunchedEffect(load, canCreateEditor, environment) {
    if (!environment.isValid) {
      return@LaunchedEffect
    }
    if (!canCreateEditor) {
      return@LaunchedEffect
    }

    if (runtime.editor == null) {
      uiState.clear()
      try {
        if (editorThemeVariant == null) editorThemeVariant = environment.themeVariant
        val editor = load.awaitEditor(environment.toViewport(), environment.themeVariant)
        while (currentLoad === load && !load.isClosed) {
          val target = currentEnvironment
          if (!target.isValid) return@LaunchedEffect
          val shouldUpdateTheme = editorThemeVariant != target.themeVariant
          if (shouldUpdateTheme) {
            val hostThemeChanged = PlatformModule.editorHost.setThemeVariant(target.themeVariant)
            if (hostThemeChanged) {
              for (registeredEditor in EditorRegistry.snapshot()) {
                if (registeredEditor !== editor) {
                  registeredEditor.enqueue(Message.System(SystemEvent.ThemeVariantChanged))
                }
              }
            }
          }
          editor.await {
            if (shouldUpdateTheme) {
              enqueue(Message.System(SystemEvent.ThemeVariantChanged))
            }
            enqueue(
              Message.System(
                SystemEvent.Resize(
                  width = target.width,
                  height = target.height,
                  scaleFactor = target.scaleFactor,
                )
              )
            )
          }
          if (shouldUpdateTheme) editorThemeVariant = target.themeVariant
          if (currentLoad !== load || load.isClosed) return@LaunchedEffect
          if (currentEnvironment != target) continue

          if (runtime.canCreateEditor && runtime.editor == null) {
            load.markEditorReady(editor)
          }
          return@LaunchedEffect
        }
      } catch (e: CancellationException) {
        throw e
      } catch (e: Throwable) {
        if (currentLoad === load && !load.isClosed) {
          runtime.reportError(e)
        }
      }
    }
  }

  Box(modifier) {
    val session = runtime.session ?: return@Box
    val editor = session.editor
    val focusManager = LocalFocusManager.current
    var previousSelection by remember(editor) { mutableStateOf(editor.selection) }
    LaunchedEffect(editor, themeVariant) {
      val changed = PlatformModule.editorHost.setThemeVariant(themeVariant)
      if (changed) {
        for (registeredEditor in EditorRegistry.snapshot()) {
          registeredEditor.enqueue(Message.System(SystemEvent.ThemeVariantChanged))
        }
      }
    }
    val autoSurroundEnabled = Preference.autoSurroundEnabled
    LaunchedEffect(autoSurroundEnabled) {
      PlatformModule.editorHost.setAutoSurroundEnabled(autoSurroundEnabled)
    }
    LaunchedEffect(editor, editor.selection, uiState.focused) {
      val currentSelection = editor.selection
      val selectionCleared = previousSelection != null && currentSelection == null
      previousSelection = currentSelection
      if (selectionCleared && uiState.focused) {
        focusManager.clearFocus()
      }
    }
    SideEffect { editor.focusManager = focusManager }
    DisposableEffect(session, uiState) {
      onDispose {
        uiState.clear()
        runtime.clear(session)
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
          enabled = editorInputEnabled,
          session = session,
          uiState = uiState,
          platform = platform,
          bringIntoViewRequests = bringIntoViewRequests,
          suppressSoftwareKeyboard = suppressSoftwareKeyboard,
          clipboard = PlatformModule.clipboard,
        )
        .focusable()
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
              if (layoutSpec is EditorDocumentLayoutSpec.Paginated) {
                EditorLineHighlightOverlay(
                  cursor = pageCursor,
                  focused = uiState.focused,
                  displayZoom = displayZoom,
                  pageWidth = size.width,
                  enabled = Preference.lineHighlightEnabled,
                )
              }
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

private data class EditorAttachEnvironment(
  val width: Float,
  val height: Float,
  val scaleFactor: Double,
  val themeVariant: ThemeVariant,
) {
  val isValid: Boolean
    get() = width > 0f && height > 0f

  fun toViewport() = Viewport(width = width, height = height, scaleFactor = scaleFactor)
}
