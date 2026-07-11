package co.typie.editor.external

import androidx.compose.foundation.Canvas
import androidx.compose.foundation.background
import androidx.compose.foundation.gestures.detectDragGestures
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.fillMaxHeight
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.offset
import androidx.compose.foundation.layout.width
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberUpdatedState
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.geometry.CornerRadius
import androidx.compose.ui.graphics.BlendMode
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.input.pointer.pointerInput
import androidx.compose.ui.input.pointer.positionChange
import androidx.compose.ui.layout.ContentScale
import androidx.compose.ui.platform.LocalDensity
import co.typie.editor.ffi.ExternalElementData
import co.typie.editor.ffi.ImageNodeAttr
import co.typie.editor.ffi.Message
import co.typie.editor.ffi.NodeAttr
import co.typie.editor.ffi.NodeOp
import co.typie.editor.runtime.LocalEditorRuntime
import co.typie.icons.Lucide
import co.typie.ui.component.Img
import co.typie.ui.component.Spinner
import co.typie.ui.theme.AppShapes
import co.typie.ui.theme.AppTheme
import coil3.compose.AsyncImage
import kotlin.math.min

private const val RESIZE_HANDLE_TOUCH_WIDTH = 24f
private const val RESIZE_HANDLE_VISUAL_WIDTH = 8f
private const val RESIZE_HANDLE_HORIZONTAL_INSET = 10f
private const val RESIZE_HANDLE_MAX_HEIGHT = 72f

@Composable
context(scope: EditorExternalElementRenderScope)
internal fun EditorImageExternalElement(
  data: ExternalElementData.Image,
  nodeId: String,
  boundsWidth: Float,
  selected: Boolean,
) {
  val density = LocalDensity.current
  val editor = LocalEditorRuntime.current.editor
  val externalElementState = LocalEditorExternalElementState.current
  val imageState = externalElementState.images
  val upload = imageState.uploads[nodeId]
  val asset = data.id?.let(imageState.assets::get)
  val hasImage = asset != null || upload != null
  val resolution = data.id?.let(externalElementState.resolutions::get)
  val missingAsset = data.id != null && asset == null && upload == null
  val unavailableAsset =
    missingAsset &&
      (resolution == EditorAssetResolution.RetryableFailure ||
        resolution == EditorAssetResolution.Unavailable)
  val resolvingAsset = missingAsset && !unavailableAsset
  val ratio = asset?.ratio ?: upload?.ratio
  var activeResizeReverse by remember(nodeId) { mutableStateOf<Boolean?>(null) }

  LaunchedEffect(selected, hasImage, boundsWidth, ratio) {
    if (!selected || !hasImage || boundsWidth <= 0f || ratio == null || ratio <= 0.0) {
      imageState.clearResizeState(nodeId)
      activeResizeReverse = null
    }
  }

  if (!hasImage) {
    ImagePlaceholder(resolvingAsset = resolvingAsset, unavailableAsset = unavailableAsset)
    return
  }

  val imageRatio = ratio ?: return
  if (boundsWidth <= 0f || imageRatio <= 0.0) {
    return
  }

  val originalWidth = (asset?.width ?: upload?.width ?: 0).toFloat()
  val nodeProportion = data.proportion.coerceIn(IMAGE_MIN_PROPORTION, IMAGE_MAX_PROPORTION)
  val draftProportion = imageState.resizeDraftProportions[nodeId]
  val displayProportion = draftProportion ?: nodeProportion.toFloat()
  val displayWidth = imageResizeWidthForProportion(displayProportion, boundsWidth, originalWidth)
  val displayHeight = displayWidth / imageRatio.toFloat()
  val imageLeft = maxOf(0f, (boundsWidth - displayWidth) / 2f)
  val handleHeight = min(RESIZE_HANDLE_MAX_HEIGHT, displayHeight / 3f)
  val handleTop = maxOf(0f, (displayHeight - handleHeight) / 2f)
  val maxHandleLeft = maxOf(0f, boundsWidth - RESIZE_HANDLE_TOUCH_WIDTH)
  val leftHandleLeft =
    (imageLeft + RESIZE_HANDLE_HORIZONTAL_INSET - RESIZE_HANDLE_TOUCH_WIDTH / 2f).coerceIn(
      0f,
      maxHandleLeft,
    )
  val rightHandleLeft =
    (imageLeft + displayWidth - RESIZE_HANDLE_HORIZONTAL_INSET - RESIZE_HANDLE_TOUCH_WIDTH / 2f)
      .coerceIn(0f, maxHandleLeft)
  val imageShape = AppShapes.rounded(scope.scaledDp(4f))
  val pointerDragScale = (scope.zoom * density.density).takeIf { it.isFinite() && it > 0f } ?: 1f
  fun commitResize() {
    val currentWidth =
      imageResizeWidthForProportion(
        proportion = imageState.resizeDraftProportions[nodeId] ?: displayProportion,
        boundsWidth = boundsWidth,
        originalWidth = originalWidth,
      )
    val nextProportion = imageResizeProportionForWidth(currentWidth, boundsWidth)
    activeResizeReverse = null
    if (nextProportion != nodeProportion) {
      editor?.sync {
        enqueue(
          Message.Node(
            NodeOp.SetAttr(
              id = nodeId,
              attr = NodeAttr.Image(ImageNodeAttr.Proportion(nextProportion)),
            )
          )
        )
      }
    }
    imageState.resizeDraftProportions.remove(nodeId)
  }

  fun updateDraftWidth(width: Float) {
    val nextWidth = clampImageResizeWidth(width, boundsWidth, originalWidth)
    imageState.resizeDraftProportions[nodeId] =
      imageResizeDraftProportionForWidth(nextWidth, boundsWidth)
  }

  Box(modifier = Modifier.fillMaxWidth().height(scope.scaledDp(displayHeight))) {
    Box(
      modifier =
        Modifier.align(Alignment.TopCenter)
          .width(scope.scaledDp(displayWidth))
          .height(scope.scaledDp(displayHeight))
          .clip(imageShape)
    ) {
      when {
        asset != null -> {
          Img(url = asset.url, modifier = Modifier.fillMaxSize(), contentScale = ContentScale.Crop)
        }
        upload != null -> {
          AsyncImage(
            model = upload.previewModel,
            contentDescription = null,
            modifier = Modifier.fillMaxSize(),
            contentScale = ContentScale.Crop,
          )
        }
      }

      if (upload != null && asset == null) {
        Box(
          modifier = Modifier.fillMaxSize().background(Color.White.copy(alpha = 0.5f)),
          contentAlignment = Alignment.Center,
        ) {
          Spinner(
            color = AppTheme.colors.textHint,
            size = scope.scaledDp(24f),
            strokeWidth = scope.scaledDp(2f),
            sweepAngle = 270f,
          )
        }
      }
    }

    if (selected && handleHeight > 0f) {
      ImageResizeHandle(
        reverse = true,
        active = activeResizeReverse == true,
        modifier =
          Modifier.offset(x = scope.scaledDp(leftHandleLeft), y = scope.scaledDp(handleTop))
            .width(scope.scaledDp(RESIZE_HANDLE_TOUCH_WIDTH))
            .height(scope.scaledDp(handleHeight)),
        onStart = {
          imageState.resizeDraftProportions[nodeId] =
            imageResizeDraftProportionForWidth(displayWidth, boundsWidth)
          activeResizeReverse = true
        },
        onDrag = { deltaX ->
          val currentWidth =
            imageResizeWidthForProportion(
              proportion = imageState.resizeDraftProportions[nodeId] ?: displayProportion,
              boundsWidth = boundsWidth,
              originalWidth = originalWidth,
            )
          updateDraftWidth(currentWidth - (deltaX / pointerDragScale) * 2f)
        },
        onEnd = ::commitResize,
      )
      ImageResizeHandle(
        reverse = false,
        active = activeResizeReverse == false,
        modifier =
          Modifier.offset(x = scope.scaledDp(rightHandleLeft), y = scope.scaledDp(handleTop))
            .width(scope.scaledDp(RESIZE_HANDLE_TOUCH_WIDTH))
            .height(scope.scaledDp(handleHeight)),
        onStart = {
          imageState.resizeDraftProportions[nodeId] =
            imageResizeDraftProportionForWidth(displayWidth, boundsWidth)
          activeResizeReverse = false
        },
        onDrag = { deltaX ->
          val currentWidth =
            imageResizeWidthForProportion(
              proportion = imageState.resizeDraftProportions[nodeId] ?: displayProportion,
              boundsWidth = boundsWidth,
              originalWidth = originalWidth,
            )
          updateDraftWidth(currentWidth + (deltaX / pointerDragScale) * 2f)
        },
        onEnd = ::commitResize,
      )
    }
  }
}

@Composable
context(scope: EditorExternalElementRenderScope)
private fun ImagePlaceholder(resolvingAsset: Boolean, unavailableAsset: Boolean) {
  EditorExternalElementPlaceholder(
    icon = Lucide.Image,
    text =
      when {
        unavailableAsset -> "이미지를 불러올 수 없어요"
        resolvingAsset -> "이미지를 불러오는 중..."
        else -> "이미지"
      },
    trailing = {
      if (resolvingAsset) {
        Spinner(
          color = AppTheme.colors.textHint,
          size = scope.scaledDp(16f),
          strokeWidth = scope.scaledDp(2f),
          sweepAngle = 270f,
        )
      }
    },
  )
}

@Composable
context(scope: EditorExternalElementRenderScope)
private fun ImageResizeHandle(
  reverse: Boolean,
  active: Boolean,
  modifier: Modifier,
  onStart: () -> Unit,
  onDrag: (Float) -> Unit,
  onEnd: () -> Unit,
) {
  val currentOnStart by rememberUpdatedState(onStart)
  val currentOnDrag by rememberUpdatedState(onDrag)
  val currentOnEnd by rememberUpdatedState(onEnd)
  Box(
    modifier =
      modifier.imageResizeHandlePointerInput(
        key = reverse,
        onStart = { currentOnStart() },
        onDrag = { currentOnDrag(it) },
        onEnd = { currentOnEnd() },
      ),
    contentAlignment = Alignment.Center,
  ) {
    Canvas(modifier = Modifier.width(scope.scaledDp(RESIZE_HANDLE_VISUAL_WIDTH)).fillMaxHeight()) {
      drawRoundRect(
        color = Color.White.copy(alpha = if (active) 0.75f else 0.55f),
        cornerRadius = CornerRadius(size.width / 2f, size.width / 2f),
        blendMode = BlendMode.Difference,
      )
    }
  }
}

internal fun Modifier.imageResizeHandlePointerInput(
  key: Any?,
  onStart: () -> Unit,
  onDrag: (Float) -> Unit,
  onEnd: () -> Unit,
): Modifier =
  pointerInput(key) {
    detectDragGestures(
      orientationLock = null,
      onDragStart = { _, _, _ -> onStart() },
      onDrag = { change, dragAmount ->
        change.consume()
        onDrag(dragAmount.x)
      },
      onDragEnd = { change ->
        val finalDeltaX = change.positionChange().x
        if (finalDeltaX != 0f) onDrag(finalDeltaX)
        onEnd()
      },
      onDragCancel = onEnd,
    )
  }
