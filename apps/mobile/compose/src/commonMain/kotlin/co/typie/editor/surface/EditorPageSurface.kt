package co.typie.editor.surface

import androidx.compose.foundation.Canvas
import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.width
import androidx.compose.runtime.Composable
import androidx.compose.runtime.remember
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.draw.drawWithContent
import androidx.compose.ui.geometry.Offset as ComposeOffset
import androidx.compose.ui.geometry.Size as ComposeSize
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.RectangleShape
import androidx.compose.ui.graphics.TransformOrigin
import androidx.compose.ui.graphics.graphicsLayer
import androidx.compose.ui.layout.Layout
import androidx.compose.ui.platform.LocalDensity
import androidx.compose.ui.unit.Dp
import androidx.compose.ui.unit.IntOffset
import androidx.compose.ui.unit.IntSize
import androidx.compose.ui.unit.dp
import co.touchlab.kermit.Logger
import co.typie.editor.LocalEditorZoomController
import co.typie.editor.PresentedFrame
import co.typie.ui.theme.AppTheme
import kotlin.math.round

private val DebugRustSurfaceTint = Color(0x220096FF)
private val DebugRustSurfaceTintAlternate = Color(0x2234C759)
private val DebugPageBottomMarginTint = Color(0x22FFD600)
private val DebugPageBoundaryTint = Color(0xE6FF3B30)
private val DebugPageBoundaryThickness = 2.dp
private val DebugMissingFrameTint = Color(0x990066FF)
private val DebugFrameAheadTint = Color(0x9930D158)

private class DebugMismatchLogHolder {
  var lastKey: String? = null
}

internal data class FrameDisplay(val frame: PresentedFrame?, val pixelSize: IntSize)

internal fun resolveFrameDisplay(
  publishedFrame: PresentedFrame?,
  desiredPixelSize: IntSize,
): FrameDisplay =
  FrameDisplay(frame = publishedFrame, pixelSize = publishedFrame?.pixelSize ?: desiredPixelSize)

@Composable
internal fun EditorPageSurface(
  page: Int,
  width: Float,
  height: Float,
  publishedVersion: Long,
  publishedFrame: PresentedFrame?,
  showChrome: Boolean,
  debugBottomMarginHeight: Float = 0f,
  showDebugOverlay: Boolean = false,
  modifier: Modifier = Modifier,
  backgroundOverlay: @Composable () -> Unit = {},
  foregroundOverlay: @Composable () -> Unit = {},
) {
  val density = LocalDensity.current
  val zoomController = LocalEditorZoomController.current
  val displayZoom = zoomController.displayZoom

  val widthDouble = width.toDouble()
  val heightDouble = height.toDouble()
  val desiredPixelSize = IntSize(round(widthDouble).toInt(), round(heightDouble).toInt())
  val debugAheadLog = remember(page) { DebugMismatchLogHolder() }
  // The published frame and page geometry come from one coherent bundle. A different
  // desired pixel size means only that a zoom/density replacement is pending, so keep
  // transforming the published frame until its replacement is delivered.
  val display = resolveFrameDisplay(publishedFrame, desiredPixelSize)
  val frame = display.frame
  val committedPixelSize = display.pixelSize

  val displayedWidthPxInt =
    round(widthDouble * density.density.toDouble() * displayZoom.toDouble())
      .toInt()
      .coerceAtLeast(1)
  val displayedHeightPxInt =
    round(heightDouble * density.density.toDouble() * displayZoom.toDouble())
      .toInt()
      .coerceAtLeast(1)
  val displayedWidthDp = Dp(displayedWidthPxInt.toFloat() / density.density)
  val displayedHeightDp = Dp(displayedHeightPxInt.toFloat() / density.density)
  val displayBottomMarginPx =
    round(debugBottomMarginHeight.toDouble() * density.density.toDouble() * displayZoom.toDouble())
      .toInt()
      .coerceIn(0, displayedHeightPxInt)
  val displayScaleX = displayedWidthPxInt.toFloat() / committedPixelSize.width
  val displayScaleY = displayedHeightPxInt.toFloat() / committedPixelSize.height
  val chromeModifier =
    if (showChrome) {
      Modifier.editorPageChromeShadow(AppTheme.themeMode)
        .background(AppTheme.colors.surfaceDefault, RectangleShape)
        .border(1.dp, AppTheme.colors.borderDefault, RectangleShape)
        .clip(RectangleShape)
    } else {
      Modifier
    }
  val debugOverlayModifier =
    if (showDebugOverlay) {
      Modifier.drawWithContent {
        drawContent()
        drawRect(if (page % 2 == 0) DebugRustSurfaceTint else DebugRustSurfaceTintAlternate)
        if (displayBottomMarginPx > 0) {
          drawRect(
            color = DebugPageBottomMarginTint,
            topLeft = ComposeOffset(x = 0f, y = size.height - displayBottomMarginPx.toFloat()),
            size = ComposeSize(width = size.width, height = displayBottomMarginPx.toFloat()),
          )
        }
        // A frame with no published pixels flashes magenta. A pixel-size mismatch is
        // expected while a zoom/density replacement is prepared: the coherent published
        // frame stays visible through the current display transform.
        if (frame == null) {
          drawRect(DebugMissingFrameTint)
        }
        // Content-ahead probe: the applied frame's tick is newer than the published
        // version this composition was built with — publication acceptance makes this
        // structurally impossible; kept as a tripwire.
        if (frame != null && frame.proof.editorRevision > publishedVersion) {
          val key = "${frame.proof.editorRevision}:${frame.proof.frameKey.value}:$publishedVersion"
          if (debugAheadLog.lastKey != key) {
            debugAheadLog.lastKey = key
            Logger.i {
              "[settle-trace] EARLY page=$page frameV=${frame.proof.editorRevision}" +
                " frameKey=${frame.proof.frameKey.value}" +
                " publishedV=$publishedVersion size=${frame.pixelSize}"
            }
          }
          drawRect(DebugFrameAheadTint)
        } else if (debugAheadLog.lastKey != null) {
          debugAheadLog.lastKey = null
          Logger.i { "[settle-trace] EARLY-CLEAR page=$page" }
        }
        val boundaryPx = DebugPageBoundaryThickness.toPx()
        drawRect(
          color = DebugPageBoundaryTint,
          topLeft = ComposeOffset.Zero,
          size = ComposeSize(width = size.width, height = boundaryPx),
        )
        drawRect(
          color = DebugPageBoundaryTint,
          topLeft = ComposeOffset(x = 0f, y = size.height - boundaryPx),
          size = ComposeSize(width = size.width, height = boundaryPx),
        )
      }
    } else {
      Modifier
    }

  Box(
    modifier =
      modifier
        .width(displayedWidthDp)
        .height(displayedHeightDp)
        .then(chromeModifier)
        .then(debugOverlayModifier)
  ) {
    backgroundOverlay()

    Layout(
      content = {
        Canvas(
          modifier =
            Modifier.graphicsLayer(
              scaleX = displayScaleX,
              scaleY = displayScaleY,
              transformOrigin = TransformOrigin(0f, 0f),
            )
        ) {
          frame?.bitmap?.let { bitmap ->
            drawImage(
              image = bitmap,
              srcOffset = IntOffset.Zero,
              srcSize = IntSize(bitmap.width, bitmap.height),
              dstOffset = IntOffset.Zero,
              dstSize = IntSize(bitmap.width, bitmap.height),
            )
          }
        }
      }
    ) { measurables, _ ->
      val placeable =
        measurables
          .single()
          .measure(
            androidx.compose.ui.unit.Constraints.fixed(
              width = committedPixelSize.width,
              height = committedPixelSize.height,
            )
          )

      layout(width = displayedWidthPxInt, height = displayedHeightPxInt) {
        placeable.place(x = 0, y = 0)
      }
    }

    foregroundOverlay()
  }
}
