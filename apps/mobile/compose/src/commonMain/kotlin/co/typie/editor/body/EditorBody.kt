package co.typie.editor.body

import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.BoxScope
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.width
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableFloatStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.layout.boundsInRoot
import androidx.compose.ui.layout.onPlaced
import androidx.compose.ui.layout.onSizeChanged
import androidx.compose.ui.platform.LocalDensity
import androidx.compose.ui.unit.dp
import co.typie.editor.EditorState
import co.typie.editor.EditorView
import co.typie.editor.LocalEditorZoomController
import co.typie.editor.PublishedBundle
import co.typie.editor.ext.unclippedBoundsInRoot
import co.typie.editor.interaction.LocalEditorInteractionScope
import co.typie.editor.overlay.editorExtensionAreaLineHighlight
import co.typie.editor.overlay.editorLineHighlightColor
import co.typie.editor.runtime.EditorUiState
import co.typie.editor.runtime.LocalEditorRuntime
import co.typie.editor.runtime.LocalEditorUiState
import co.typie.editor.scroll.EditorAutoScrollPolicy
import co.typie.editor.scroll.EditorVisibleArea
import co.typie.editor.sync.DocumentEditorLoad
import co.typie.screen.editor.editor.overlay.EditorSelectionHandleOverlay
import co.typie.screen.editor.editor.overlay.EditorTableCellSelectionOverlay
import co.typie.screen.editor.editor.overlay.EditorTableColumnResizeOverlay
import co.typie.storage.Preference

private val DebugTopPaddingColor = Color(0x22FF5ACD)
private val DebugBottomPaddingColor = Color(0x22FF8A00)
private val DebugExtensionFillColor = Color(0x2200B8D4)

@Composable
internal fun EditorBody(
  load: DocumentEditorLoad,
  publishedBundle: PublishedBundle?,
  visibleArea: EditorVisibleArea,
  layoutSpec: EditorDocumentLayoutSpec,
  autoScrollPolicy: EditorAutoScrollPolicy,
  modifier: Modifier = Modifier,
  editorInputEnabled: Boolean = true,
  suppressSoftwareKeyboard: Boolean = false,
  showDebugBodyOverlay: Boolean = false,
  showDebugSurfaceOverlay: Boolean = false,
  overlay: @Composable BoxScope.(EditorBodyGeometry, EditorState) -> Unit = { _, _ -> },
) {
  val density = LocalDensity.current
  val displayZoom = LocalEditorZoomController.current.displayZoom
  val editor = LocalEditorRuntime.current.editor
  val uiState = LocalEditorUiState.current
  val interactionScope = LocalEditorInteractionScope.current
  var bodyContentHeight by remember { mutableFloatStateOf(0f) }
  val presentedBundle = publishedBundle
  val presentedState = presentedBundle?.snapshot ?: EditorState.Initial
  val pageSizes = presentedState.pageSizes
  val geometry =
    resolveEditorBodyGeometry(
      visibleArea = visibleArea,
      layoutSpec = layoutSpec,
      pageSizes = pageSizes,
      displayZoom = displayZoom,
    )
  val cursor = presentedState.cursor
  val extensionAreaFillSpacerHeight =
    remember(geometry.minimumBodyHeight, bodyContentHeight) {
      resolveExtensionAreaFillSpacerHeight(
        minimumHeight = geometry.minimumBodyHeight,
        bodyContentHeight = bodyContentHeight,
      )
    }
  val interactionSurfaceModifier =
    Modifier.fillMaxWidth()
      .trackEditorInteractionSurfaceBounds(uiState = uiState, density = density.density)
      .run {
        if (layoutSpec is EditorDocumentLayoutSpec.Continuous) {
          // Continuous spans the full extension area and stays as a draw modifier on this surface.
          // A measured Canvas(matchParentSize()) child exceeded Compose's finite Constraints on
          // long documents.
          editorExtensionAreaLineHighlight(
            cursor = cursor,
            focused = uiState.focused,
            editorBounds = { uiState.editorBoundsInContainer },
            viewportTransform = { uiState.resolveViewportTransform(pageSizes) },
            enabled = Preference.lineHighlightEnabled,
            color = editorLineHighlightColor(),
          )
        } else {
          this
        }
      }

  Box(modifier = modifier.fillMaxWidth()) {
    Box(modifier = interactionSurfaceModifier) {
      Box(modifier = Modifier.fillMaxWidth(), contentAlignment = Alignment.TopCenter) {
        Column(
          modifier =
            Modifier.run {
              if (geometry.pageColumnWidth > 0f) {
                width(geometry.pageColumnWidth.dp)
              } else {
                fillMaxWidth()
              }
            }
        ) {
          Column(
            modifier =
              Modifier.fillMaxWidth().onSizeChanged { size ->
                bodyContentHeight = size.height / density.density
              }
          ) {
            if (geometry.topSpacerHeight > 0f) {
              Spacer(
                modifier =
                  Modifier.fillMaxWidth()
                    .height(geometry.topSpacerHeight.dp)
                    .debugBackground(enabled = showDebugBodyOverlay, color = DebugTopPaddingColor)
              )
            }

            Box(
              modifier =
                Modifier.fillMaxWidth()
                  .trackEditorContentBounds(uiState = uiState, density = density.density)
            ) {
              EditorView(
                load = load,
                publishedBundle = presentedBundle,
                layoutSpec = layoutSpec,
                viewportWidth = geometry.visibleBodySize.width,
                viewportHeight = geometry.visibleBodySize.height,
                modifier = Modifier.fillMaxWidth(),
                editorInputEnabled = editorInputEnabled,
                suppressSoftwareKeyboard = suppressSoftwareKeyboard,
                showDebugSurfaceOverlay = showDebugSurfaceOverlay,
              )
            }

            if (autoScrollPolicy.bottomPadding > 0f) {
              Spacer(
                modifier =
                  Modifier.fillMaxWidth()
                    .height(autoScrollPolicy.bottomPadding.dp)
                    .debugBackground(
                      enabled = showDebugBodyOverlay,
                      color = DebugBottomPaddingColor,
                    )
              )
            }
          }

          if (extensionAreaFillSpacerHeight > 0f) {
            Spacer(
              modifier =
                Modifier.fillMaxWidth()
                  .height(extensionAreaFillSpacerHeight.dp)
                  .debugBackground(enabled = showDebugBodyOverlay, color = DebugExtensionFillColor)
            )
          }
        }
      }
      if (editor != null) {
        EditorTableColumnResizeOverlay(
          editor = editor,
          uiState = uiState,
          geometry = interactionScope,
          presentation = interactionScope.controller.tableColumnResizePresentation,
          resolvePlacement = interactionScope.controller::resolveTableColumnResizePlacement,
        )
        EditorTableCellSelectionOverlay(
          state = presentedState,
          uiState = uiState,
          density = density.density,
          pagePresented = { page -> presentedBundle?.frames?.containsKey(page) == true },
        )
        EditorSelectionHandleOverlay(
          state = presentedState,
          uiState = uiState,
          density = density.density,
          pagePresented = { page -> presentedBundle?.frames?.containsKey(page) == true },
        )
      }
    }

    Box(modifier = Modifier.fillMaxSize()) { overlay(geometry, presentedState) }
  }
}

internal fun Modifier.trackEditorInteractionSurfaceBounds(
  uiState: EditorUiState,
  density: Float,
): Modifier = onPlaced { coordinates ->
  uiState.updateInteractionSurfaceBounds(
    boundsInRoot = coordinates.unclippedBoundsInRoot(),
    density = density,
  )
}

internal fun Modifier.trackEditorContentBounds(uiState: EditorUiState, density: Float): Modifier =
  onPlaced { coordinates ->
    uiState.updateEditorBounds(
      boundsInRoot = coordinates.unclippedBoundsInRoot(),
      clippedBoundsInRoot = coordinates.boundsInRoot(),
      density = density,
    )
  }

private fun Modifier.debugBackground(enabled: Boolean, color: Color): Modifier =
  if (enabled) {
    background(color)
  } else {
    this
  }

internal fun resolveExtensionAreaFillSpacerHeight(
  minimumHeight: Float,
  bodyContentHeight: Float,
): Float = (minimumHeight - bodyContentHeight).coerceAtLeast(0f)
