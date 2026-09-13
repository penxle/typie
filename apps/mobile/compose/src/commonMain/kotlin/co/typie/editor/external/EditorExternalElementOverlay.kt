package co.typie.editor.external

import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.offset
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.key
import androidx.compose.runtime.mutableFloatStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.TransformOrigin
import androidx.compose.ui.layout.layout
import androidx.compose.ui.layout.onSizeChanged
import androidx.compose.ui.platform.LocalDensity
import androidx.compose.ui.unit.Constraints
import androidx.compose.ui.unit.IntOffset
import androidx.compose.ui.unit.constrainHeight
import androidx.compose.ui.unit.constrainWidth
import co.typie.editor.EditorTheme
import co.typie.editor.currentEditorThemeVariant
import co.typie.editor.ffi.ExternalElement
import co.typie.editor.ffi.ExternalElementData
import co.typie.editor.ffi.Message
import co.typie.editor.ffi.SystemEvent
import co.typie.editor.runtime.LocalEditorRuntime
import co.typie.editor.runtime.LocalEditorUiState
import co.typie.icons.Lucide
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
      key(element.node) { EditorExternalElement(element = element, displayZoom = displayZoom) }
    }
  }
}

@Composable
private fun EditorExternalElement(element: ExternalElement, displayZoom: Float) {
  if (element.bounds.width <= 0f) {
    return
  }

  val editor = LocalEditorRuntime.current.editor ?: return
  val imageState = LocalEditorExternalElementState.current.images
  val imageSize =
    (element.data as? ExternalElementData.Image)?.let {
      imageState.displaySize(element.node, it, element.bounds.width)
    }
  val uiState = LocalEditorUiState.current
  val density = LocalDensity.current
  val zoom = if (displayZoom.isFinite() && displayZoom > 0f) displayZoom else 1f
  // Images report intrinsic geometry, so keep measuring them at the visible size for decoding.
  val imageZoom = if (imageSize != null) zoom else 1f
  val layerZoom = zoom / imageZoom
  var reportedHeight by remember(element.node) { mutableFloatStateOf(Float.NaN) }
  val themeVariant = currentEditorThemeVariant()
  val selectionColor =
    remember(themeVariant) { EditorTheme.resolve(themeVariant).colors.getValue("selection") }
  val selectionAlpha = if (uiState.focused) SELECTION_FOCUSED_ALPHA else SELECTION_UNFOCUSED_ALPHA

  fun reportHeight(height: Float) {
    if (height <= 0f || !height.isFinite()) return
    val unchanged =
      if (imageSize != null) reportedHeight == height else abs(reportedHeight - height) < 0.5f
    if (unchanged) return
    reportedHeight = height
    editor.runCallback {
      editor.enqueue(Message.System(SystemEvent.SetExternalHeight(element.node, height)))
    }
  }

  LaunchedEffect(imageSize?.height) { imageSize?.height?.let(::reportHeight) }

  Box(
    Modifier.offset {
        IntOffset(
          x = (element.bounds.x * zoom * density.density).roundToInt(),
          y = (element.bounds.y * zoom * density.density).roundToInt(),
        )
      }
      .layout { measurable, constraints ->
        val width = (element.bounds.width * imageZoom * density.density).roundToInt()
        val placeable = measurable.measure(Constraints.fixedWidth(width))
        layout(
          constraints.constrainWidth((placeable.width * layerZoom).roundToInt()),
          constraints.constrainHeight((placeable.height * layerZoom).roundToInt()),
        ) {
          placeable.placeWithLayer(0, 0) {
            transformOrigin = TransformOrigin(0f, 0f)
            scaleX = layerZoom
            scaleY = layerZoom
            alpha = if (reportedHeight.isNaN()) 0f else 1f
          }
        }
      }
      .onSizeChanged { size ->
        if (imageSize == null) {
          reportHeight(size.height.toFloat() / density.density)
        }
      }
  ) {
    when (val data = element.data) {
      is ExternalElementData.Image ->
        EditorImageExternalElement(
          data = data,
          nodeId = element.node,
          size = imageSize,
          zoom = imageZoom,
        )
      is ExternalElementData.File -> EditorFileExternalElement(data = data, nodeId = element.node)
      is ExternalElementData.Embed ->
        EditorEmbedExternalElement(data = data, nodeId = element.node, zoom = zoom)
      is ExternalElementData.Archived ->
        EditorExternalElementPlaceholder(icon = Lucide.Archive, text = "보관된 블록")
    }

    if (element.isSelected) {
      Box(Modifier.matchParentSize().background(selectionColor.copy(alpha = selectionAlpha)))
    }
  }
}
