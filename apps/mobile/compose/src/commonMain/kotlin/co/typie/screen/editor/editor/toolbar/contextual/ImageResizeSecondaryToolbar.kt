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
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import co.typie.editor.EditorSurfaceUnavailableException
import co.typie.editor.external.EditorImageResizeDraft
import co.typie.editor.external.IMAGE_MAX_PROPORTION
import co.typie.editor.external.IMAGE_MIN_PROPORTION
import co.typie.editor.external.LocalEditorExternalElementState
import co.typie.editor.external.imageResizeDisplayPercent
import co.typie.editor.external.imageResizeHeightForProportion
import co.typie.editor.external.imageResizeProportionRange
import co.typie.editor.ffi.CommandOutcome
import co.typie.editor.ffi.ExternalElementData
import co.typie.editor.ffi.ImageNodeAttr
import co.typie.editor.ffi.Message
import co.typie.editor.ffi.NodeAttr
import co.typie.editor.ffi.NodeOp
import co.typie.editor.ffi.SystemEvent
import co.typie.editor.runtime.LocalEditorRuntime
import co.typie.screen.editor.editor.toolbar.ToolbarLabelTextStyle
import co.typie.screen.editor.editor.toolbar.ToolbarPageVerticalPadding
import co.typie.screen.editor.editor.toolbar.ToolbarSecondaryContentStartInset
import co.typie.ui.component.Slider
import co.typie.ui.component.Text
import co.typie.ui.theme.AppTheme
import kotlin.math.roundToInt
import kotlinx.coroutines.CoroutineStart
import kotlinx.coroutines.Job

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
  val coroutineScope = rememberCoroutineScope()
  val imageState = LocalEditorExternalElementState.current.images
  val externalElement =
    editor?.publishedState?.externalElements?.firstOrNull { element ->
      element.node == nodeId && element.data is ExternalElementData.Image
    }
  val image = externalElement?.data as? ExternalElementData.Image
  val imageId = image?.id
  val asset = imageId?.let(imageState.assets::get)
  val boundsWidth = externalElement?.bounds?.width ?: 0f
  val imageRatio = asset?.ratio?.toFloat() ?: 0f

  if (
    image == null ||
      asset == null ||
      boundsWidth <= 0f ||
      !imageRatio.isFinite() ||
      imageRatio <= 0f
  ) {
    LaunchedEffect(nodeId, imageId, boundsWidth, imageRatio) {
      imageState.clearResizeState(nodeId)
      onClose()
    }
    return
  }

  val originalWidth = asset.width.toFloat()
  val publishedRange =
    imageResizeProportionRange(boundsWidth = boundsWidth, originalWidth = originalWidth)
  val nodeProportion = image.proportion.coerceIn(IMAGE_MIN_PROPORTION, IMAGE_MAX_PROPORTION)
  var publicationWaitJob by remember(nodeId) { mutableStateOf<Job?>(null) }
  val draft = imageState.resizeDrafts[nodeId]
  val range =
    draft?.let {
      imageResizeProportionRange(boundsWidth = it.boundsWidth, originalWidth = it.originalWidth)
    } ?: publishedRange
  val currentProportion =
    (draft?.proportion ?: nodeProportion.toFloat()).coerceIn(
      range.first.toFloat(),
      range.last.toFloat(),
    )
  val currentPercent =
    imageResizeDisplayPercent(
      currentProportion,
      draft?.boundsWidth ?: boundsWidth,
      draft?.originalWidth ?: originalWidth,
    )

  DisposableEffect(editor, nodeId) {
    onDispose {
      val previousWait = publicationWaitJob
      publicationWaitJob = null
      previousWait?.cancel()
      imageState.clearResizeState(nodeId)
    }
  }

  fun clearDraft() {
    imageState.clearResizeState(nodeId)
  }

  fun beginDraft(value: Float) {
    val previousWait = publicationWaitJob
    publicationWaitJob = null
    previousWait?.cancel()
    imageState.resizeDrafts[nodeId] =
      EditorImageResizeDraft(
        proportion = value.coerceIn(publishedRange.first.toFloat(), publishedRange.last.toFloat()),
        boundsWidth = boundsWidth,
        originalWidth = originalWidth,
      )
  }

  fun updateDraft(value: Float) {
    val currentDraft =
      imageState.resizeDrafts[nodeId]
        ?: EditorImageResizeDraft(
          proportion = value,
          boundsWidth = boundsWidth,
          originalWidth = originalWidth,
        )
    val stableRange =
      imageResizeProportionRange(
        boundsWidth = currentDraft.boundsWidth,
        originalWidth = currentDraft.originalWidth,
      )
    val next = value.coerceIn(stableRange.first.toFloat(), stableRange.last.toFloat())
    imageState.resizeDrafts[nodeId] = currentDraft.copy(proportion = next)
  }

  fun commit(value: Float) {
    val stableDraft = imageState.resizeDrafts[nodeId]
    val stableRange =
      stableDraft?.let {
        imageResizeProportionRange(boundsWidth = it.boundsWidth, originalWidth = it.originalWidth)
      } ?: publishedRange
    val next = value.roundToInt().coerceIn(stableRange.first, stableRange.last)
    if (next == nodeProportion) {
      clearDraft()
      return
    }
    val stableBoundsWidth = stableDraft?.boundsWidth ?: boundsWidth
    val stableOriginalWidth = stableDraft?.originalWidth ?: originalWidth
    val finalHeight =
      imageResizeHeightForProportion(
        proportion = next.toFloat(),
        boundsWidth = stableBoundsWidth,
        originalWidth = stableOriginalWidth,
        imageRatio = imageRatio,
      )
    imageState.resizeDrafts[nodeId] =
      EditorImageResizeDraft(
        proportion = next.toFloat(),
        boundsWidth = stableBoundsWidth,
        originalWidth = stableOriginalWidth,
      )
    val update = editor.runCallback {
      editor.updateNow {
        enqueue(
          Message.Node(
            NodeOp.SetAttr(id = nodeId, attr = NodeAttr.Image(ImageNodeAttr.Proportion(next)))
          )
        )
        enqueue(Message.System(SystemEvent.SetExternalHeight(nodeId, finalHeight)))
      }
    }
    if (
      update == null || update.commandOutcomes.any { outcome -> outcome is CommandOutcome.Rejected }
    ) {
      clearDraft()
      return
    }

    val waitJob =
      editor.launchEffect(coroutineScope = coroutineScope, start = CoroutineStart.LAZY) {
        try {
          update.awaitPublishedInEffect()
        } catch (_: EditorSurfaceUnavailableException) {
          // A replacement surface can publish a newer revision; this draft is no longer needed.
        }
      }
    waitJob.invokeOnCompletion {
      if (publicationWaitJob === waitJob) {
        publicationWaitJob = null
        clearDraft()
      }
    }
    publicationWaitJob = waitJob
    waitJob.start()
  }

  fun cancelDraft() {
    clearDraft()
  }

  ImageResizeSecondaryToolbarSurface(onClose = onClose, modifier = modifier) {
    Slider(
      value = currentProportion,
      range = range.first.toFloat()..range.last.toFloat(),
      onDragStart = ::beginDraft,
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
