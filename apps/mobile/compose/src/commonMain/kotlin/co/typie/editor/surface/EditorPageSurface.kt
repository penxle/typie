package co.typie.editor.surface

import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.width
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableIntStateOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
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
import co.touchlab.kermit.Logger
import co.typie.editor.EditorSurfaceUnavailableException
import co.typie.editor.LocalEditorZoomController
import co.typie.editor.PresentedFrame
import co.typie.editor.SurfaceConfiguration
import co.typie.editor.SurfaceSessionHandle
import co.typie.editor.ffi.FrameKey
import co.typie.editor.render.RenderCanvas
import co.typie.editor.render.RenderCanvasLifecycle
import co.typie.editor.runSurfaceCleanup
import co.typie.editor.runtime.LocalEditorRuntime
import co.typie.ui.theme.AppTheme
import kotlin.math.round
import kotlinx.coroutines.CancellationException
import kotlinx.coroutines.channels.BufferOverflow
import kotlinx.coroutines.flow.MutableSharedFlow

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
  val renderZoom = zoomController.renderZoom
  val scaleFactor = density.density.toDouble() * renderZoom.toDouble()
  val runtime = LocalEditorRuntime.current
  val editor = runtime.editor ?: runtime.failedEditor ?: return

  val trigger = remember {
    MutableSharedFlow<FrameKey>(replay = 1, onBufferOverflow = BufferOverflow.DROP_OLDEST)
  }
  val wakeDelivery =
    remember(trigger) {
      { frameKey: FrameKey ->
        trigger.tryEmit(frameKey)
        Unit
      }
    }
  var surfaceInstance by remember(editor, page) { mutableIntStateOf(0) }
  // A replacement is successful only after its frame is published. Failure before that
  // means this document host cannot create a usable replacement target.
  var replacingUnavailableTarget by remember(editor, page) { mutableStateOf(false) }
  LaunchedEffect(publishedFrame?.proof?.surfaceKey) {
    if (publishedFrame != null) replacingUnavailableTarget = false
  }

  val widthDouble = width.toDouble()
  val heightDouble = height.toDouble()
  val configuration =
    SurfaceConfiguration(width = widthDouble, height = heightDouble, scaleFactor = scaleFactor)
  val desiredPixelSize =
    IntSize(
      width = round(configuration.width * configuration.scaleFactor).toInt().coerceAtLeast(1),
      height = round(configuration.height * configuration.scaleFactor).toInt().coerceAtLeast(1),
    )
  val debugAheadLog = remember(editor, page) { DebugMismatchLogHolder() }
  // The published frame and page geometry come from one coherent bundle. A different
  // desired pixel size means only that a zoom/density replacement is pending, so keep
  // transforming the published frame until its replacement is delivered.
  val display = resolveFrameDisplay(publishedFrame, desiredPixelSize)
  val frame = display.frame
  val committedPixelSize = display.pixelSize
  var renderActive by remember(editor, page) { mutableStateOf(false) }

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
        if (renderActive && frame == null) {
          drawRect(DebugMissingFrameTint)
        }
        // Content-ahead probe: the applied frame's tick is newer than the published
        // version this composition was built with — structurally impossible with the
        // version condition in the gate; kept as a tripwire.
        if (renderActive && frame != null && frame.proof.editorRevision > publishedVersion) {
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
        .editorPageRenderGate { renderActive = it }
        .then(chromeModifier)
        .then(debugOverlayModifier)
  ) {
    backgroundOverlay()

    if (renderActive) {
      Layout(
        content = {
          RenderCanvasLifecycle(owner = editor, page = page, instance = surfaceInstance) {
            var surfaceHandle by remember { mutableStateOf<Long?>(null) }
            var surfaceSession by remember { mutableStateOf<SurfaceSessionHandle?>(null) }
            val attachSurface = { handle: Long ->
              surfaceHandle = handle
              surfaceSession =
                editor.attachSurface(
                  page,
                  handle,
                  widthDouble,
                  heightDouble,
                  scaleFactor,
                  wakeDelivery,
                )
            }
            val appliedPageExists = editor.appliedState.pageSizes.getOrNull(page) != null
            LaunchedEffect(appliedPageExists, surfaceSession?.isRetired, surfaceHandle) {
              val handle = surfaceHandle
              // Applied shrink retires only the editor session; this canvas still owns its buffer.
              if (appliedPageExists && surfaceSession?.isRetired == true && handle != null) {
                try {
                  attachSurface(handle)
                } catch (error: CancellationException) {
                  throw error
                } catch (error: Throwable) {
                  editor.surfaceDeliveryFailed(page, session = null, error)
                }
              }
            }
            RenderCanvas(
              modifier =
                Modifier.graphicsLayer(
                  scaleX = displayScaleX,
                  scaleY = displayScaleY,
                  transformOrigin = TransformOrigin(0f, 0f),
                ),
              desiredPixelSize = desiredPixelSize,
              configuration = configuration,
              frame = frame?.bitmap,
              retainedFrames = { editor.retainedFrames(page) },
              trigger = trigger,
              onAttach = attachSurface,
              onDetach = { releaseBuffer ->
                runSurfaceCleanup { surfaceSession?.detach(releaseBuffer) ?: releaseBuffer() }
              },
              onResize = { surfaceSession?.requestResize(configuration) },
              onFrame = { bitmap, size, editorRevision, frameKey ->
                surfaceSession?.let { session ->
                  editor.deliverFrame(
                    session = session,
                    bitmap = bitmap,
                    pixelSize = size,
                    editorRevision = editorRevision,
                    frameKey = frameKey,
                  )
                }
              },
              onFrameUnavailable = { frameKey ->
                surfaceSession?.let { session -> editor.surfaceUnavailable(session, frameKey) }
              },
              onTargetUnavailable = { frameKey ->
                val replaceOrFail = {
                  if (replacingUnavailableTarget) {
                    runtime.reportError(editor, EditorSurfaceUnavailableException(page))
                  } else {
                    replacingUnavailableTarget = true
                    surfaceInstance += 1
                  }
                }
                val session = surfaceSession
                if (session != null && frameKey != null) {
                  editor.surfaceTargetUnavailable(session, frameKey) { accepted ->
                    if (accepted) replaceOrFail()
                  }
                } else {
                  replaceOrFail()
                }
              },
              onFailure = { error -> editor.surfaceDeliveryFailed(page, surfaceSession, error) },
            )
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
    }

    foregroundOverlay()
  }
}
