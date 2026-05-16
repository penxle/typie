package co.typie.editor.external

import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.heightIn
import androidx.compose.foundation.layout.offset
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.layout.widthIn
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableFloatStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.graphics.graphicsLayer
import androidx.compose.ui.layout.onSizeChanged
import androidx.compose.ui.platform.LocalDensity
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.IntOffset
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import co.typie.editor.EditorTheme
import co.typie.editor.EditorThemeVariant
import co.typie.editor.ffi.ExternalElement
import co.typie.editor.ffi.ExternalElementData
import co.typie.editor.ffi.Message
import co.typie.editor.ffi.SystemEvent
import co.typie.editor.runtime.LocalEditorRuntime
import co.typie.icons.Lucide
import co.typie.ui.component.Text
import co.typie.ui.icon.Icon
import co.typie.ui.icon.IconData
import co.typie.ui.theme.AppShapes
import co.typie.ui.theme.AppTheme
import kotlin.math.abs
import kotlin.math.roundToInt

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
  val density = LocalDensity.current
  val safeZoom = if (displayZoom.isFinite() && displayZoom > 0f) displayZoom else 1f
  var reportedHeight by remember(element.nodeId) { mutableFloatStateOf(Float.NaN) }
  val content = element.data.content()
  val shape = AppShapes.rounded(4.dp * safeZoom)
  val selectionColor = remember {
    EditorTheme.resolve(EditorThemeVariant.LightWhite).colors.getValue("selection")
  }

  Box(
    Modifier.offset {
        IntOffset(
          x = (element.bounds.x * safeZoom * density.density).roundToInt(),
          y = (element.bounds.y * safeZoom * density.density).roundToInt(),
        )
      }
      .width((element.bounds.width * safeZoom).dp)
      .graphicsLayer { alpha = if (reportedHeight.isNaN()) 0f else 1f }
      .onSizeChanged { size ->
        val height = size.height.toFloat() / density.density / safeZoom
        if (height <= 0f || !height.isFinite()) {
          return@onSizeChanged
        }
        if (!reportedHeight.isNaN() && abs(reportedHeight - height) < 0.5f) {
          return@onSizeChanged
        }
        reportedHeight = height
        editor.enqueue(Message.System(SystemEvent.SetExternalHeight(element.nodeId, height)))
      }
  ) {
    Row(
      modifier =
        Modifier.widthIn(min = 0.dp)
          .heightIn(min = 48.dp * safeZoom)
          .fillMaxWidth()
          .clip(shape)
          .background(AppTheme.colors.surfaceInset, shape)
          .border(1.dp, AppTheme.colors.borderDefault, shape)
          .padding(horizontal = 14.dp * safeZoom, vertical = 12.dp * safeZoom),
      verticalAlignment = Alignment.CenterVertically,
    ) {
      Icon(
        icon = content.icon,
        contentDescription = null,
        modifier = Modifier.size(20.dp * safeZoom),
        tint = AppTheme.colors.textMuted,
      )
      Text(
        text = content.label,
        modifier = Modifier.padding(start = 12.dp * safeZoom).weight(1f),
        color = AppTheme.colors.textMuted,
        style = AppTheme.typography.body.copy(fontSize = (14f * safeZoom).sp),
        maxLines = 1,
        overflow = TextOverflow.Ellipsis,
      )
    }

    if (element.isSelected) {
      Box(Modifier.matchParentSize().background(selectionColor.copy(alpha = 64f / 255f)))
    }
  }
}

private data class ExternalElementContent(val icon: IconData, val label: String)

private fun ExternalElementData.content(): ExternalElementContent =
  when (this) {
    is ExternalElementData.Image -> ExternalElementContent(Lucide.Image, "이미지")
    is ExternalElementData.File -> ExternalElementContent(Lucide.File, "파일")
    is ExternalElementData.Embed -> ExternalElementContent(Lucide.FileUp, "임베드")
    is ExternalElementData.Archived -> ExternalElementContent(Lucide.Archive, "보관된 블록")
  }
