package co.typie.editor.body

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
import androidx.compose.ui.layout.boundsInRoot
import androidx.compose.ui.layout.onGloballyPositioned
import androidx.compose.ui.layout.onSizeChanged
import androidx.compose.ui.platform.LocalDensity
import androidx.compose.ui.unit.dp
import co.typie.editor.EditorView
import co.typie.editor.ffi.Doc
import co.typie.editor.ffi.Selection
import co.typie.editor.runtime.LocalEditorUiState
import co.typie.screen.editor.editor.layout.EditorBodyGeometry
import kotlin.math.max

@Composable
internal fun EditorBody(
  doc: Doc,
  selection: Selection,
  geometry: EditorBodyGeometry,
  modifier: Modifier = Modifier,
  overlay: @Composable BoxScope.() -> Unit = {},
) {
  val density = LocalDensity.current
  val uiState = LocalEditorUiState.current
  val activeBottomPadding = max(geometry.defaultBottomPadding, geometry.typewriterBottomPadding)
  var coreTrackHeight by remember { mutableFloatStateOf(0f) }
  val extensionFillHeight =
    remember(geometry.minimumBodyHeight, coreTrackHeight) {
      resolveEditorBodyFillHeight(
        minimumHeight = geometry.minimumBodyHeight,
        coreTrackHeight = coreTrackHeight,
      )
    }

  Box(modifier = modifier.fillMaxWidth()) {
    EditorExtensionArea(
      modifier =
        Modifier.fillMaxWidth().onGloballyPositioned { coordinates ->
          uiState.updateExtensionAreaBounds(
            boundsInRoot = coordinates.boundsInRoot(),
            density = density.density,
          )
        }
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
                coreTrackHeight = size.height / density.density
              }
          ) {
            if (geometry.defaultTopPadding > 0f) {
              Spacer(modifier = Modifier.fillMaxWidth().height(geometry.defaultTopPadding.dp))
            }

            Box(
              modifier =
                Modifier.fillMaxWidth().onGloballyPositioned { coordinates ->
                  uiState.updateEditorBounds(
                    boundsInRoot = coordinates.boundsInRoot(),
                    density = density.density,
                  )
                }
            ) {
              EditorView(
                doc = doc,
                selection = selection,
                viewportWidth = geometry.visibleBodyRect.width,
                viewportHeight = geometry.visibleBodyRect.height,
                modifier = Modifier.fillMaxWidth(),
              )
            }

            if (activeBottomPadding > 0f) {
              Spacer(modifier = Modifier.fillMaxWidth().height(activeBottomPadding.dp))
            }
          }

          if (extensionFillHeight > 0f) {
            Spacer(modifier = Modifier.fillMaxWidth().height(extensionFillHeight.dp))
          }
        }
      }
    }

    Box(modifier = Modifier.fillMaxSize(), content = overlay)
  }
}

internal fun resolveEditorBodyFillHeight(minimumHeight: Float, coreTrackHeight: Float): Float =
  max(0f, minimumHeight - coreTrackHeight)
