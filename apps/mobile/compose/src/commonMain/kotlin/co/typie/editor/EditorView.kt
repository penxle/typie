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
import androidx.compose.runtime.derivedStateOf
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
import co.typie.editor.input.LocalEditorIncomingContentHandler
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
  val incomingContentHandler = LocalEditorIncomingContentHandler.current
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
            EditorRegistry.commitResourceUpdate {
              PlatformModule.editorHost.setThemeVariant(target.themeVariant)
            }
          }
          editor.update {
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
    val session = runtime.session
    val editor = session?.editor ?: runtime.failedEditor ?: return@Box
    val sessionActive = session != null

    if (session != null) {
      val visualHostToken = remember(editor) { Any() }
      DisposableEffect(editor, visualHostToken) {
        val activated =
          try {
            editor.activateVisualHost(visualHostToken)
            true
          } catch (error: Throwable) {
            if (!editor.terminal) throw error
            false
          }
        onDispose { if (activated) editor.deactivateVisualHost(visualHostToken) }
      }
      val focusManager = LocalFocusManager.current
      val publishedSelection by
        remember(editor) { derivedStateOf { editor.publishedState.selection } }
      var previousSelection by remember(editor) { mutableStateOf(publishedSelection) }
      LaunchedEffect(editor, themeVariant) {
        EditorRegistry.commitResourceUpdate {
          PlatformModule.editorHost.setThemeVariant(themeVariant)
        }
      }
      val autoSurroundEnabled = Preference.autoSurroundEnabled
      LaunchedEffect(autoSurroundEnabled) {
        EditorRegistry.commitResourceUpdate {
          PlatformModule.editorHost.setAutoSurroundEnabled(autoSurroundEnabled)
        }
      }
      LaunchedEffect(editor, publishedSelection, uiState.focused) {
        val selectionCleared = previousSelection != null && publishedSelection == null
        previousSelection = publishedSelection
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
    }

    val editorInteractionModifier =
      if (session != null) {
        Modifier.focusRequester(editor.focusRequester)
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
            incomingContentHandler = incomingContentHandler,
          )
          .focusable()
      } else {
        Modifier
      }

    Box(Modifier.fillMaxWidth().then(editorInteractionModifier)) {
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
        val publishedBundle = editor.publishedBundle
        val publishedState = publishedBundle?.snapshot ?: EditorState.Initial
        val publishedVersion = publishedState.version
        publishedState.pageSizes.forEachIndexed { index, size ->
          val pageCursor = publishedState.cursor?.takeIf { sessionActive && it.pageIdx == index }
          val pageExternalElements =
            if (sessionActive) publishedState.externalElements.filter { it.pageIdx == index }
            else emptyList()
          EditorPageSurface(
            page = index,
            width = size.width,
            height = size.height,
            publishedVersion = publishedVersion,
            publishedFrame = publishedBundle?.frames?.get(index),
            showChrome = showPageChrome,
            debugBottomMarginHeight =
              when (layoutSpec) {
                is EditorDocumentLayoutSpec.Paginated -> layoutSpec.pageMarginBottom
                is EditorDocumentLayoutSpec.Continuous -> 0f
              },
            showDebugOverlay = showDebugSurfaceOverlay,
            backgroundOverlay = {
              if (sessionActive && layoutSpec is EditorDocumentLayoutSpec.Paginated) {
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
              if (sessionActive) {
                EditorExternalElementOverlay(
                  elements = pageExternalElements,
                  displayZoom = displayZoom,
                )
                EditorCursorOverlay(
                  cursor = pageCursor,
                  focused = uiState.focused,
                  displayZoom = displayZoom,
                )
              }
            },
            modifier =
              if (sessionActive) {
                Modifier.editorPagePositionTracker(
                  uiState = uiState,
                  page = index,
                  density = density.density,
                )
              } else {
                Modifier
              },
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
