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
import co.typie.editor.EditorView
import co.typie.editor.ext.unclippedBoundsInRoot
import co.typie.editor.interaction.LocalEditorInteractionScope
import co.typie.editor.overlay.EditorExtensionAreaLineHighlightOverlay
import co.typie.editor.runtime.EditorUiState
import co.typie.editor.runtime.LocalEditorRuntime
import co.typie.editor.runtime.LocalEditorUiState
import co.typie.editor.scroll.EditorAutoScrollPolicy
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
  geometry: EditorBodyGeometry,
  layoutSpec: EditorDocumentLayoutSpec,
  autoScrollPolicy: EditorAutoScrollPolicy,
  modifier: Modifier = Modifier,
  interactionModifier: Modifier = Modifier,
  editorInputEnabled: Boolean = true,
  suppressSoftwareKeyboard: Boolean = false,
  showDebugBodyOverlay: Boolean = false,
  showDebugSurfaceOverlay: Boolean = false,
  overlay: @Composable BoxScope.() -> Unit = {},
) {
  val density = LocalDensity.current
  val editor = LocalEditorRuntime.current.editor
  val uiState = LocalEditorUiState.current
  val interactionScope = LocalEditorInteractionScope.current
  var bodyContentHeight by remember { mutableFloatStateOf(0f) }
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

  Box(modifier = modifier.fillMaxWidth()) {
    Box(modifier = interactionSurfaceModifier.then(interactionModifier)) {
      if (layoutSpec is EditorDocumentLayoutSpec.Continuous) {
        EditorExtensionAreaLineHighlightOverlay(
          cursor = editor?.cursor,
          focused = uiState.focused,
          editorBounds = { uiState.editorBoundsInContainer },
          viewportTransform = { uiState.resolveViewportTransform(editor?.pageSizes.orEmpty()) },
          enabled = Preference.lineHighlightEnabled,
          modifier = Modifier.matchParentSize(),
        )
      }
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
                layoutSpec = layoutSpec,
                viewportWidth = geometry.visibleBodySize.width,
                viewportHeight = geometry.visibleBodySize.height,
                modifier = Modifier.fillMaxWidth(),
                editorInputEnabled = editorInputEnabled,
                suppressSoftwareKeyboard = suppressSoftwareKeyboard,
                showDebugSurfaceOverlay = showDebugSurfaceOverlay,
              )
            }

            if (autoScrollPolicy.bottomSpacerHeight > 0f) {
              Spacer(
                modifier =
                  Modifier.fillMaxWidth()
                    .height(autoScrollPolicy.bottomSpacerHeight.dp)
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
        )
        EditorTableCellSelectionOverlay(
          editor = editor,
          uiState = uiState,
          density = density.density,
        )
        EditorSelectionHandleOverlay(editor = editor, uiState = uiState, density = density.density)
      }
    }

    Box(modifier = Modifier.fillMaxSize(), content = overlay)
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
