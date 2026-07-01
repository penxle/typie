package co.typie.editor.external

import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.offset
import androidx.compose.foundation.layout.width
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableFloatStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.graphicsLayer
import androidx.compose.ui.layout.onSizeChanged
import androidx.compose.ui.platform.LocalDensity
import androidx.compose.ui.unit.IntOffset
import androidx.compose.ui.unit.dp
import co.typie.editor.EditorTheme
import co.typie.editor.currentEditorThemeVariant
import co.typie.editor.ffi.ExternalElement
import co.typie.editor.ffi.ExternalElementData
import co.typie.editor.ffi.Message
import co.typie.editor.ffi.SystemEvent
import co.typie.editor.runtime.LocalEditorRuntime
import co.typie.editor.runtime.LocalEditorUiState
import co.typie.icons.Lucide
import co.typie.ui.theme.AppShapes
import kotlin.math.abs
import kotlin.math.roundToInt

private const val SELECTION_FOCUSED_ALPHA = 77f / 255f
private const val SELECTION_UNFOCUSED_ALPHA = 48f / 255f

@Composable
internal fun EditorExternalElementOverlay(
  elements: List<ExternalElement>,
  displayZoom: Float,
  modifier: Modifier = Modifier,
) {
  if (elements.isEmpty()) {
    return
  }

  Box(modifier.fillMaxSize()) {
    for (element in elements) {
      EditorExternalElement(element = element, displayZoom = displayZoom)
    }
  }
}

@Composable
private fun EditorExternalElement(element: ExternalElement, displayZoom: Float) {
  if (element.bounds.width <= 0f) {
    return
  }

  val editor = LocalEditorRuntime.current.editor ?: return
  val uiState = LocalEditorUiState.current
  val density = LocalDensity.current
  val zoom = if (displayZoom.isFinite() && displayZoom > 0f) displayZoom else 1f
  var reportedHeight by remember(element.node) { mutableFloatStateOf(Float.NaN) }
  val renderScope =
    remember(zoom) {
      EditorExternalElementRenderScope(zoom = zoom, shape = AppShapes.rounded(4.dp * zoom))
    }
  val themeVariant = currentEditorThemeVariant()
  val selectionColor =
    remember(themeVariant) { EditorTheme.resolve(themeVariant).colors.getValue("selection") }
  val selectionAlpha = if (uiState.focused) SELECTION_FOCUSED_ALPHA else SELECTION_UNFOCUSED_ALPHA

  Box(
    Modifier.offset {
        IntOffset(
          x = (element.bounds.x * zoom * density.density).roundToInt(),
          y = (element.bounds.y * zoom * density.density).roundToInt(),
        )
      }
      .width((element.bounds.width * zoom).dp)
      .graphicsLayer { alpha = if (reportedHeight.isNaN()) 0f else 1f }
      .onSizeChanged { size ->
        val height = size.height.toFloat() / density.density / zoom
        if (height <= 0f || !height.isFinite()) {
          return@onSizeChanged
        }
        if (!reportedHeight.isNaN() && abs(reportedHeight - height) < 0.5f) {
          return@onSizeChanged
        }
        reportedHeight = height
        editor.enqueue(Message.System(SystemEvent.SetExternalHeight(element.node, height)))
      }
  ) {
    context(renderScope) {
      when (val data = element.data) {
        is ExternalElementData.Image ->
          EditorImageExternalElement(
            data = data,
            nodeId = element.node,
            boundsWidth = element.bounds.width,
            selected = element.isSelected,
          )
        is ExternalElementData.File -> EditorFileExternalElement(data = data, nodeId = element.node)
        is ExternalElementData.Embed ->
          EditorEmbedExternalElement(data = data, nodeId = element.node)
        is ExternalElementData.Archived ->
          EditorExternalElementPlaceholder(icon = Lucide.Archive, text = "보관된 블록")
      }
    }

    if (element.isSelected) {
      Box(Modifier.matchParentSize().background(selectionColor.copy(alpha = selectionAlpha)))
    }
  }
}
