package co.typie.screen.editor.editor.toolbar.contextual

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.RowScope
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.width
import androidx.compose.runtime.Composable
import androidx.compose.runtime.DisposableEffect
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import co.typie.editor.external.IMAGE_MAX_PROPORTION
import co.typie.editor.external.IMAGE_MIN_PROPORTION
import co.typie.editor.external.LocalEditorExternalElementState
import co.typie.editor.external.imageResizeProportionRange
import co.typie.editor.ffi.ExternalElementData
import co.typie.editor.ffi.ImageNodeAttr
import co.typie.editor.ffi.Message
import co.typie.editor.ffi.NodeAttr
import co.typie.editor.ffi.NodeOp
import co.typie.editor.runtime.LocalEditorRuntime
import co.typie.screen.editor.editor.toolbar.ToolbarLabelTextStyle
import co.typie.screen.editor.editor.toolbar.ToolbarPageVerticalPadding
import co.typie.screen.editor.editor.toolbar.ToolbarSecondaryContentStartInset
import co.typie.ui.component.Slider
import co.typie.ui.component.Text
import co.typie.ui.theme.AppTheme
import kotlin.math.roundToInt

private val ImageResizeToolbarItemGap = 8.dp
private val ImageResizeToolbarEndPadding = 12.dp

@Composable
internal fun ImageResizeSecondaryToolbar(
  nodeId: String,
  onClose: () -> Unit,
  modifier: Modifier = Modifier,
) {
  val runtime = LocalEditorRuntime.current
  val editor = runtime.editor
  val imageState = LocalEditorExternalElementState.current.images
  val externalElement =
    editor?.externalElements?.firstOrNull { element ->
      element.node == nodeId && element.data is ExternalElementData.Image
    }
  val image = externalElement?.data as? ExternalElementData.Image
  val imageId = image?.id
  val asset = imageId?.let(imageState.assets::get)
  val boundsWidth = externalElement?.bounds?.width ?: 0f

  if (image == null || asset == null || boundsWidth <= 0f) {
    LaunchedEffect(nodeId, imageId, boundsWidth) {
      imageState.clearResizeState(nodeId)
      onClose()
    }
    return
  }

  val range =
    imageResizeProportionRange(boundsWidth = boundsWidth, originalWidth = asset.width.toFloat())
  val nodeProportion = image.proportion.coerceIn(IMAGE_MIN_PROPORTION, IMAGE_MAX_PROPORTION)
  val currentProportion =
    (imageState.resizeDraftProportions[nodeId] ?: nodeProportion.toFloat()).coerceIn(
      range.first.toFloat(),
      range.last.toFloat(),
    )
  val currentPercent = currentProportion.roundToInt()

  DisposableEffect(nodeId) { onDispose { imageState.resizeDraftProportions.remove(nodeId) } }

  fun updateDraft(value: Float) {
    val next = value.coerceIn(range.first.toFloat(), range.last.toFloat())
    imageState.resizeDraftProportions[nodeId] = next
  }

  fun startDraft() {
    imageState.resizeDraftProportions[nodeId] = currentProportion
  }

  fun commit(value: Float) {
    val next = value.roundToInt().coerceIn(range.first, range.last)
    if (next != nodeProportion) {
      editor.sync {
        enqueue(
          Message.Node(
            NodeOp.SetAttr(id = nodeId, attr = NodeAttr.Image(ImageNodeAttr.Proportion(next)))
          )
        )
      }
    }
    imageState.resizeDraftProportions.remove(nodeId)
  }

  fun cancelDraft() {
    imageState.resizeDraftProportions.remove(nodeId)
  }

  ImageResizeSecondaryToolbarSurface(onClose = onClose, modifier = modifier) {
    Slider(
      value = currentProportion,
      range = range.first.toFloat()..range.last.toFloat(),
      onDragStart = ::startDraft,
      onDrag = ::updateDraft,
      onDragEnd = ::commit,
      onDragCancel = ::cancelDraft,
      thumbSize = 20.dp,
      trackHeight = 6.dp,
      fillColor = AppTheme.colors.textDefault.copy(alpha = 0.78f),
      modifier = Modifier.weight(1f).height(30.dp),
    )
    Text(
      text = "$currentPercent%",
      modifier = Modifier.width(48.dp),
      style = ToolbarLabelTextStyle,
      color = AppTheme.colors.textDefault,
      maxLines = 1,
      overflow = TextOverflow.Ellipsis,
      textAlign = TextAlign.End,
    )
  }
}

@Composable
private fun ImageResizeSecondaryToolbarSurface(
  onClose: () -> Unit,
  modifier: Modifier = Modifier,
  content: @Composable RowScope.() -> Unit,
) {
  ToolbarSecondarySurface(
    onClose = onClose,
    closeContentDescription = "이미지 폭 조정 닫기",
    modifier = modifier,
  ) {
    Row(
      modifier =
        Modifier.fillMaxSize()
          .padding(
            start = ToolbarSecondaryContentStartInset + ImageResizeToolbarItemGap,
            top = ToolbarPageVerticalPadding,
            end = ImageResizeToolbarEndPadding,
            bottom = ToolbarPageVerticalPadding,
          ),
      verticalAlignment = Alignment.CenterVertically,
      horizontalArrangement = Arrangement.spacedBy(ImageResizeToolbarItemGap),
    ) {
      content()
    }
  }
}
