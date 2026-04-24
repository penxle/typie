package co.typie.editor.surface

import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.width
import androidx.compose.runtime.Composable
import androidx.compose.runtime.DisposableEffect
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
import androidx.compose.ui.unit.dp
import co.typie.editor.LocalEditorZoomController
import co.typie.editor.ffi.EditorEvent
import co.typie.editor.render.RenderCanvas
import co.typie.editor.runtime.LocalEditorRuntime
import co.typie.ui.theme.AppTheme
import co.typie.ui.theme.shadow
import kotlin.math.round
import kotlinx.coroutines.channels.BufferOverflow
import kotlinx.coroutines.flow.MutableSharedFlow

private val DebugRustSurfaceTint = Color(0x220096FF)
private val DebugPageBottomMarginTint = Color(0x22FFD600)

@Composable
internal fun EditorPageSurface(
  page: Int,
  width: Float,
  height: Float,
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
  val renderZoom = zoomController.renderZoom
  val scaleFactor = density.density.toDouble() * renderZoom.toDouble()
  val editor = LocalEditorRuntime.current.editor ?: return

  val trigger = remember {
    MutableSharedFlow<Unit>(replay = 1, onBufferOverflow = BufferOverflow.DROP_OLDEST)
  }
  val render =
    remember(editor, page) {
      {
        editor.renderSurface(page)
        trigger.tryEmit(Unit)
      }
    }

  val widthDouble = width.toDouble()
  val heightDouble = height.toDouble()
  val displayWidthPx = round(widthDouble * density.density.toDouble() * displayZoom.toDouble())
  val displayHeightPx = round(heightDouble * density.density.toDouble() * displayZoom.toDouble())
  val renderWidthPx = round(widthDouble * density.density.toDouble() * renderZoom.toDouble())
  val renderHeightPx = round(heightDouble * density.density.toDouble() * renderZoom.toDouble())
  val displayWidthPxInt = displayWidthPx.toInt().coerceAtLeast(1)
  val displayHeightPxInt = displayHeightPx.toInt().coerceAtLeast(1)
  val renderWidthPxInt = renderWidthPx.toInt().coerceAtLeast(1)
  val renderHeightPxInt = renderHeightPx.toInt().coerceAtLeast(1)
  val displayBottomMarginPx =
    round(debugBottomMarginHeight.toDouble() * density.density.toDouble() * displayZoom.toDouble())
      .toInt()
      .coerceIn(0, displayHeightPxInt)
  val displayWidthDp = Dp((displayWidthPx / density.density.toDouble()).toFloat())
  val displayHeightDp = Dp((displayHeightPx / density.density.toDouble()).toFloat())
  val renderScale =
    if (renderZoom > 0f) {
      displayZoom / renderZoom
    } else {
      1f
    }
  val chromeModifier =
    if (showChrome) {
      Modifier.shadow(AppTheme.shadows.md, RectangleShape)
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
        drawRect(DebugRustSurfaceTint)
        if (displayBottomMarginPx > 0) {
          drawRect(
            color = DebugPageBottomMarginTint,
            topLeft = ComposeOffset(x = 0f, y = size.height - displayBottomMarginPx.toFloat()),
            size = ComposeSize(width = size.width, height = displayBottomMarginPx.toFloat()),
          )
        }
      }
    } else {
      Modifier
    }

  Box(
    modifier =
      modifier
        .width(displayWidthDp)
        .height(displayHeightDp)
        .then(chromeModifier)
        .then(debugOverlayModifier)
  ) {
    backgroundOverlay()

    Layout(
      content = {
        RenderCanvas(
          modifier =
            Modifier.graphicsLayer(
              scaleX = renderScale,
              scaleY = renderScale,
              transformOrigin = TransformOrigin(0f, 0f),
            ),
          trigger = trigger,
          onAttach = { handle ->
            editor.attachSurface(page, handle, widthDouble, heightDouble, scaleFactor)
            render()
          },
          onDetach = { editor.detachSurface(page) },
          onResize = {
            editor.resizeSurface(page, widthDouble, heightDouble, scaleFactor)
            render()
          },
        )
      }
    ) { measurables, _ ->
      val placeable =
        measurables
          .single()
          .measure(
            androidx.compose.ui.unit.Constraints.fixed(
              width = renderWidthPxInt,
              height = renderHeightPxInt,
            )
          )

      layout(width = displayWidthPxInt, height = displayHeightPxInt) {
        placeable.place(x = 0, y = 0)
      }
    }

    foregroundOverlay()
  }

  DisposableEffect(editor, page) {
    val off = editor.on<EditorEvent.RenderInvalidated> { _, _ -> render() }

    onDispose { off() }
  }
}
