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
import androidx.compose.runtime.mutableFloatStateOf
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
import androidx.compose.ui.layout.ContentScale
import androidx.compose.ui.platform.LocalDensity
import co.typie.editor.ffi.ExternalElementData
import co.typie.editor.ffi.Message
import co.typie.editor.ffi.NodeOp
import co.typie.editor.ffi.PlainNode
import co.typie.editor.runtime.LocalEditorRuntime
import co.typie.icons.Lucide
import co.typie.ui.component.Img
import co.typie.ui.component.Spinner
import co.typie.ui.theme.AppShapes
import co.typie.ui.theme.AppTheme
import coil3.compose.AsyncImage
import kotlin.math.max
import kotlin.math.min
import kotlin.math.roundToInt

private const val IMAGE_MIN_WIDTH = 100f
private const val IMAGE_MIN_PROPORTION = 10
private const val IMAGE_MAX_PROPORTION = 100
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
  val imageState = LocalEditorExternalElementState.current.images
  val upload = imageState.uploads[nodeId]
  val asset = data.id?.let(imageState.assets::get)
  val hasImage = asset != null || upload != null
  val resolvingAsset = data.id != null && asset == null && upload == null

  if (!hasImage) {
    ImagePlaceholder(resolvingAsset = resolvingAsset)
    return
  }

  val ratio = asset?.ratio ?: upload?.ratio ?: return
  if (boundsWidth <= 0f || ratio <= 0.0) {
    return
  }

  val originalWidth = (asset?.width ?: upload?.width ?: 0).toFloat()
  val nodeProportion = data.proportion.coerceIn(IMAGE_MIN_PROPORTION, IMAGE_MAX_PROPORTION)
  var draftWidth by remember(nodeId) { mutableFloatStateOf(Float.NaN) }
  var pendingProportion by remember(nodeId) { mutableStateOf<Int?>(null) }
  var activeResizeReverse by remember(nodeId) { mutableStateOf<Boolean?>(null) }
  LaunchedEffect(selected, hasImage, boundsWidth) {
    if (!selected || !hasImage || boundsWidth <= 0f) {
      draftWidth = Float.NaN
      pendingProportion = null
      activeResizeReverse = null
    }
  }
  LaunchedEffect(nodeProportion, pendingProportion) {
    val pending = pendingProportion ?: return@LaunchedEffect
    if (pending == nodeProportion) {
      pendingProportion = null
    }
  }
  val displayProportion = pendingProportion ?: nodeProportion
  val displayWidth =
    if (draftWidth.isFinite()) {
      draftWidth
    } else {
      clampImageWidth(boundsWidth * (displayProportion / 100f), boundsWidth, originalWidth)
    }
  val displayHeight = displayWidth / ratio.toFloat()
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
    val currentWidth = if (draftWidth.isFinite()) draftWidth else displayWidth
    val nextProportion =
      ((currentWidth / boundsWidth) * 100)
        .roundToInt()
        .coerceIn(IMAGE_MIN_PROPORTION, IMAGE_MAX_PROPORTION)
    draftWidth = Float.NaN
    activeResizeReverse = null
    if (nextProportion != nodeProportion) {
      pendingProportion = nextProportion
      editor?.enqueue(
        Message.Node(
          NodeOp.SetAttrs(
            id = nodeId,
            attrs = PlainNode.Image(id = data.id, proportion = nextProportion),
          )
        )
      )
    } else {
      pendingProportion = null
    }
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
            model = upload.bytes,
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
          draftWidth = displayWidth
          activeResizeReverse = true
        },
        onDrag = { deltaX ->
          pendingProportion = null
          val current = if (draftWidth.isFinite()) draftWidth else displayWidth
          draftWidth =
            clampImageWidth(current - (deltaX / pointerDragScale) * 2f, boundsWidth, originalWidth)
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
          draftWidth = displayWidth
          activeResizeReverse = false
        },
        onDrag = { deltaX ->
          pendingProportion = null
          val current = if (draftWidth.isFinite()) draftWidth else displayWidth
          draftWidth =
            clampImageWidth(current + (deltaX / pointerDragScale) * 2f, boundsWidth, originalWidth)
        },
        onEnd = ::commitResize,
      )
    }
  }
}

@Composable
context(scope: EditorExternalElementRenderScope)
private fun ImagePlaceholder(resolvingAsset: Boolean) {
  EditorExternalElementPlaceholder(
    icon = Lucide.Image,
    text = if (resolvingAsset) "이미지를 불러오는 중..." else "이미지",
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
      modifier.pointerInput(reverse) {
        detectDragGestures(
          onDragStart = { currentOnStart() },
          onDrag = { change, dragAmount ->
            change.consume()
            currentOnDrag(dragAmount.x)
          },
          onDragEnd = { currentOnEnd() },
          onDragCancel = { currentOnEnd() },
        )
      },
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

private fun clampImageWidth(width: Float, boundsWidth: Float, originalWidth: Float): Float {
  val maxWidth = min(boundsWidth, if (originalWidth > 0f) originalWidth else boundsWidth)
  val requestedMin = max(boundsWidth * (IMAGE_MIN_PROPORTION / 100f), IMAGE_MIN_WIDTH)
  val minWidth = min(requestedMin, maxWidth)
  return width.coerceIn(minWidth, maxWidth)
}
