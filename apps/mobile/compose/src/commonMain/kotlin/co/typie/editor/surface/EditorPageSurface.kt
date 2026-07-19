package co.typie.editor.surface

import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.width
import androidx.compose.runtime.Composable
import androidx.compose.runtime.DisposableEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableFloatStateOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberUpdatedState
import androidx.compose.runtime.setValue
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
import androidx.compose.ui.unit.IntSize
import androidx.compose.ui.unit.dp
import co.typie.editor.LocalEditorZoomController
import co.typie.editor.SurfaceConfiguration
import co.typie.editor.SurfaceSessionHandle
import co.typie.editor.ffi.EditorEvent
import co.typie.editor.render.RenderCanvas
import co.typie.editor.runtime.LocalEditorRuntime
import co.typie.ui.theme.AppTheme
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
    MutableSharedFlow<Long>(replay = 1, onBufferOverflow = BufferOverflow.DROP_OLDEST)
  }
  val onPresented =
    remember(trigger) {
      { version: Long ->
        trigger.tryEmit(version)
        Unit
      }
    }
  var surfaceSession by remember(editor, page) { mutableStateOf<SurfaceSessionHandle?>(null) }

  val widthDouble = width.toDouble()
  val heightDouble = height.toDouble()
  val configuration =
    SurfaceConfiguration(width = widthDouble, height = heightDouble, scaleFactor = scaleFactor)
  val desiredPixelSize =
    IntSize(
      width = round(configuration.width * configuration.scaleFactor).toInt().coerceAtLeast(1),
      height = round(configuration.height * configuration.scaleFactor).toInt().coerceAtLeast(1),
    )
  var committedPixelSize by remember { mutableStateOf(desiredPixelSize) }
  var committedRenderZoom by remember { mutableFloatStateOf(renderZoom) }
  val currentRenderZoom by rememberUpdatedState(renderZoom)
  val render =
    remember(editor, page, onPresented) { { surfaceSession?.requestRender(onPresented) } }
  var renderActive by remember(editor, page) { mutableStateOf(false) }

  val safeCommittedRenderZoom = if (committedRenderZoom > 0f) committedRenderZoom else 1f
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
  val committedRenderScale = displayZoom / safeCommittedRenderZoom
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
        .width(displayedWidthDp)
        .height(displayedHeightDp)
        .editorPageRenderGate { renderActive = it }
        .then(chromeModifier)
        .then(debugOverlayModifier)
  ) {
    backgroundOverlay()

    if (renderActive) {
      Layout(
        content = {
          RenderCanvas(
            modifier =
              Modifier.graphicsLayer(
                scaleX = committedRenderScale,
                scaleY = committedRenderScale,
                transformOrigin = TransformOrigin(0f, 0f),
              ),
            desiredPixelSize = desiredPixelSize,
            configuration = configuration,
            trigger = trigger,
            onAttach = { handle ->
              surfaceSession =
                editor.attachSurface(page, handle, widthDouble, heightDouble, scaleFactor)
            },
            onDetach = { releaseBuffer ->
              surfaceSession?.detach(releaseBuffer) ?: releaseBuffer()
              surfaceSession = null
            },
            onResize = { surfaceSession?.requestResize(configuration, onPresented) },
            onBitmapCommitted = { size, version ->
              committedPixelSize = size
              committedRenderZoom = currentRenderZoom
              editor.onPageSettled(page, version)
            },
          )
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
    }

    foregroundOverlay()
  }

  DisposableEffect(editor, page) {
    val off = editor.on<EditorEvent.RenderInvalidated> { _, _ -> render() }

    onDispose { off() }
  }
}
