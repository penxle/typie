package co.typie.editor.surface

import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.width
import androidx.compose.runtime.Composable
import androidx.compose.runtime.DisposableEffect
import androidx.compose.runtime.getValue
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
import androidx.compose.ui.graphics.ImageBitmap
import androidx.compose.ui.graphics.RectangleShape
import androidx.compose.ui.graphics.TransformOrigin
import androidx.compose.ui.graphics.graphicsLayer
import androidx.compose.ui.layout.Layout
import androidx.compose.ui.platform.LocalDensity
import androidx.compose.ui.unit.Dp
import androidx.compose.ui.unit.IntSize
import androidx.compose.ui.unit.dp
import co.touchlab.kermit.Logger
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
private val DebugRustSurfaceTintAlternate = Color(0x2234C759)
private val DebugPageBottomMarginTint = Color(0x22FFD600)
private val DebugPageBoundaryTint = Color(0xE6FF3B30)
private val DebugPageBoundaryThickness = 2.dp
private val DebugFrameSizeMismatchTint = Color(0x99FF9500)
private val DebugMissingFrameTint = Color(0x990066FF)
private val DebugFrameAheadTint = Color(0x9930D158)

private data class PresentedFrame(
  val bitmap: ImageBitmap,
  val pixelSize: IntSize,
  val renderZoom: Float,
  val version: Long,
)

private class AppliedFrameHolder {
  var frame: PresentedFrame? = null
}

private class DebugMismatchLogHolder {
  var lastKey: String? = null
}

@Composable
internal fun EditorPageSurface(
  page: Int,
  width: Float,
  height: Float,
  publishedVersion: Long,
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
  val runtime = LocalEditorRuntime.current
  val editor = runtime.editor ?: runtime.failedEditor ?: return

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
  val pendingFramesState = remember(editor, page) { mutableStateOf(emptyList<PresentedFrame>()) }
  val appliedFrameHolder = remember(editor, page) { AppliedFrameHolder() }
  val debugMismatchLog = remember(editor, page) { DebugMismatchLogHolder() }
  val debugAheadLog = remember(editor, page) { DebugMismatchLogHolder() }
  // A committed frame becomes visible only once the page box and the published state
  // agree with it. Both conditions compare against values derived from this
  // composition's own parameters — the geometry from width/height and the version from
  // publishedVersion — so pixels, box, overlays, and the caret-reveal scroll can only
  // ever change together. Reading the editor state directly here instead would race:
  // the publish lands on a background thread, and a frame-delivery recomposition can
  // observe the new version before the parent's parameter propagation. Without the
  // version condition, a same-size frame would apply at delivery and move its content
  // one edit ahead of the publish (and of the reveal scroll that lands with it).
  // The last frame of each recent size is retained: under rapid edits the next frame can
  // arrive before the composition for the previous publish runs, and a single slot would
  // lose the frame the current box needs.
  val frame =
    pendingFramesState.value.firstOrNull {
      it.pixelSize == desiredPixelSize && it.version <= publishedVersion
    } ?: appliedFrameHolder.frame
  appliedFrameHolder.frame = frame
  val committedPixelSize = frame?.pixelSize ?: desiredPixelSize
  val committedRenderZoom = frame?.renderZoom ?: renderZoom
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
        drawRect(if (page % 2 == 0) DebugRustSurfaceTint else DebugRustSurfaceTintAlternate)
        if (displayBottomMarginPx > 0) {
          drawRect(
            color = DebugPageBottomMarginTint,
            topLeft = ComposeOffset(x = 0f, y = size.height - displayBottomMarginPx.toFloat()),
            size = ComposeSize(width = size.width, height = displayBottomMarginPx.toFloat()),
          )
        }
        // Composition-consistency probes: a frame where the shown pixels do not match the
        // published page geometry flashes orange; a frame with no pixels at all flashes
        // magenta. Render-gated pages draw no canvas at all, so their retained frame
        // reference is not a visible mismatch.
        if (!renderActive) {
          debugMismatchLog.lastKey = null
        } else if (frame == null) {
          drawRect(DebugMissingFrameTint)
        } else if (frame.pixelSize != desiredPixelSize) {
          val key = "${frame.version}:${frame.pixelSize}:$desiredPixelSize"
          if (debugMismatchLog.lastKey != key) {
            debugMismatchLog.lastKey = key
            Logger.i {
              "[settle-trace] ORANGE page=$page frame=${frame.pixelSize} frameV=${frame.version}" +
                " desired=$desiredPixelSize stateV=${editor.state.version}" +
                " param=${width}x$height sf=$scaleFactor renderZoom=$renderZoom"
            }
          }
          drawRect(DebugFrameSizeMismatchTint)
        } else if (debugMismatchLog.lastKey != null) {
          debugMismatchLog.lastKey = null
          Logger.i { "[settle-trace] ORANGE-CLEAR page=$page frameV=${frame.version}" }
        }
        // Content-ahead probe: the applied frame's tick is newer than the published
        // version this composition was built with — structurally impossible with the
        // version condition in the gate; kept as a tripwire.
        if (renderActive && frame != null && frame.version > publishedVersion) {
          val key = "${frame.version}:$publishedVersion"
          if (debugAheadLog.lastKey != key) {
            debugAheadLog.lastKey = key
            Logger.i {
              "[settle-trace] EARLY page=$page frameV=${frame.version}" +
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
            frame = frame?.bitmap,
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
            onFrame = { bitmap, size, version ->
              val next =
                PresentedFrame(
                  bitmap = bitmap,
                  pixelSize = size,
                  renderZoom = currentRenderZoom,
                  version = version,
                )
              // Two frames per size: at publish time the qualifying frame is the one
              // whose version the publish covers, which a newest-only rule would have
              // already replaced when the next same-size frame landed first.
              val existing = pendingFramesState.value
              pendingFramesState.value =
                listOf(next) +
                  existing.filter { it.pixelSize == size }.take(1) +
                  existing.filter { it.pixelSize != size }.take(2)
              editor.onPageSettled(page, version)
            },
            onFrameSkipped = { version -> editor.onPageSettled(page, version) },
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
