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
import androidx.compose.ui.geometry.Rect
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.layout.LayoutCoordinates
import androidx.compose.ui.layout.onGloballyPositioned
import androidx.compose.ui.layout.onSizeChanged
import androidx.compose.ui.layout.positionInRoot
import androidx.compose.ui.platform.LocalDensity
import androidx.compose.ui.unit.dp
import co.typie.editor.EditorView
import co.typie.editor.ffi.Doc
import co.typie.editor.ffi.Selection
import co.typie.editor.runtime.LocalEditorUiState
import co.typie.editor.scroll.EditorAutoScrollPolicy

private val DebugTopPaddingColor = Color(0x22FF5ACD)
private val DebugBottomPaddingColor = Color(0x22FF8A00)
private val DebugExtensionFillColor = Color(0x2200B8D4)

@Composable
internal fun EditorBody(
  doc: Doc,
  initialSelection: Selection,
  geometry: EditorBodyGeometry,
  layoutSpec: EditorDocumentLayoutSpec,
  autoScrollPolicy: EditorAutoScrollPolicy,
  modifier: Modifier = Modifier,
  textInputSessionEnabled: Boolean = true,
  showDebugBodyOverlay: Boolean = false,
  showDebugSurfaceOverlay: Boolean = false,
  overlay: @Composable BoxScope.() -> Unit = {},
) {
  val density = LocalDensity.current
  val uiState = LocalEditorUiState.current
  val extensionForwardingEnabled = layoutSpec is EditorDocumentLayoutSpec.Continuous
  var bodyContentHeight by remember { mutableFloatStateOf(0f) }
  val extensionAreaFillSpacerHeight =
    remember(geometry.minimumBodyHeight, bodyContentHeight) {
      resolveExtensionAreaFillSpacerHeight(
        minimumHeight = geometry.minimumBodyHeight,
        bodyContentHeight = bodyContentHeight,
      )
    }
  val containerModifier =
    Modifier.fillMaxWidth().onGloballyPositioned { coordinates ->
      uiState.updateExtensionAreaBounds(
        boundsInRoot = coordinates.unclippedBoundsInRoot(),
        density = density.density,
      )
    }

  Box(modifier = modifier.fillMaxWidth()) {
    EditorExtensionArea(
      forwardingEnabled = extensionForwardingEnabled,
      modifier = containerModifier,
    ) {
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
                Modifier.fillMaxWidth().onGloballyPositioned { coordinates ->
                  uiState.updateEditorBounds(
                    boundsInRoot = coordinates.unclippedBoundsInRoot(),
                    density = density.density,
                  )
                }
            ) {
              EditorView(
                doc = doc,
                initialSelection = initialSelection,
                layoutSpec = layoutSpec,
                viewportWidth = geometry.visibleBodySize.width,
                viewportHeight = geometry.visibleBodySize.height,
                modifier = Modifier.fillMaxWidth(),
                textInputSessionEnabled = textInputSessionEnabled,
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
    }

    Box(modifier = Modifier.fillMaxSize(), content = overlay)
  }
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

private fun LayoutCoordinates.unclippedBoundsInRoot(): Rect {
  val position = positionInRoot()
  return Rect(
    left = position.x,
    top = position.y,
    right = position.x + size.width,
    bottom = position.y + size.height,
  )
}
