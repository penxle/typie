package co.typie.editor.surface

import androidx.compose.runtime.Composable
import androidx.compose.runtime.DisposableEffect
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.key
import androidx.compose.runtime.mutableIntStateOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.unit.IntSize
import co.typie.editor.Editor
import co.typie.editor.EditorSurfaceUnavailableException
import co.typie.editor.SurfaceConfiguration
import co.typie.editor.SurfaceSessionHandle
import co.typie.editor.ffi.FrameKey
import co.typie.editor.render.RenderFrameProducer
import co.typie.editor.runSurfaceCleanup
import kotlin.math.round
import kotlinx.coroutines.CancellationException
import kotlinx.coroutines.channels.BufferOverflow
import kotlinx.coroutines.flow.MutableSharedFlow

@Composable
internal fun EditorSurfaceHost(
  editor: Editor,
  scaleFactor: Double,
  onDeactivate: () -> Unit = {},
  onPublicationFailure: (Long) -> Unit = {},
  onFailure: (Throwable) -> Unit,
) {
  val hostLifetime = remember(editor) { Any() }
  DisposableEffect(editor, hostLifetime) {
    val active =
      editor.runCallback {
        editor.activateVisualHost(hostLifetime, onPublicationFailure)
        true
      } ?: false
    onDispose {
      if (active) editor.deactivateVisualHost(hostLifetime)
      onDeactivate()
    }
  }

  val requiredPages = editor.surfacePageRequirements
  requiredPages.forEach { page ->
    val size = editor.appliedState.pageSizes.getOrNull(page) ?: return@forEach
    val configuration =
      SurfaceConfiguration(
        width = size.width.toDouble(),
        height = size.height.toDouble(),
        scaleFactor = scaleFactor,
      )
    key(editor, hostLifetime, page) {
      EditorSurfaceProducer(
        editor = editor,
        page = page,
        configuration = configuration,
        onFailure = onFailure,
      )
    }
  }
}

@Composable
private fun EditorSurfaceProducer(
  editor: Editor,
  page: Int,
  configuration: SurfaceConfiguration,
  onFailure: (Throwable) -> Unit,
) {
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
  var instance by remember(editor, page) { mutableIntStateOf(0) }
  var replacingUnavailableTarget by remember(editor, page) { mutableStateOf(false) }
  val displayedFrame = editor.publishedBundle?.frames?.get(page)
  LaunchedEffect(displayedFrame?.proof?.surfaceKey) {
    if (displayedFrame != null) replacingUnavailableTarget = false
  }

  val desiredPixelSize =
    IntSize(
      width = round(configuration.width * configuration.scaleFactor).toInt().coerceAtLeast(1),
      height = round(configuration.height * configuration.scaleFactor).toInt().coerceAtLeast(1),
    )
  key(instance) {
    var surfaceHandle by remember { mutableStateOf<Long?>(null) }
    var surfaceSession by remember { mutableStateOf<SurfaceSessionHandle?>(null) }
    val attachSurface: (Long) -> Unit = attachSurface@{ handle ->
      if (page !in editor.surfacePageRequirements) return@attachSurface
      surfaceHandle = handle
      surfaceSession =
        editor.attachSurface(
          page = page,
          handle = handle,
          width = configuration.width,
          height = configuration.height,
          scaleFactor = configuration.scaleFactor,
          wakeDelivery = wakeDelivery,
        )
    }
    LaunchedEffect(surfaceSession?.isRetired, surfaceHandle) {
      val handle = surfaceHandle
      if (
        surfaceSession?.isRetired == true &&
          handle != null &&
          page in editor.surfacePageRequirements
      ) {
        try {
          attachSurface(handle)
        } catch (error: CancellationException) {
          throw error
        } catch (error: Throwable) {
          editor.surfaceDeliveryFailed(page, session = null, error)
        }
      }
    }
    RenderFrameProducer(
      desiredPixelSize = desiredPixelSize,
      configuration = configuration,
      displayedFrame = displayedFrame?.bitmap,
      retainedFrames = { editor.retainedFrames(page) },
      trigger = trigger,
      onAttach = attachSurface,
      onDetach = { releaseBuffer ->
        runSurfaceCleanup { surfaceSession?.detach(releaseBuffer) ?: releaseBuffer() }
      },
      onResize = { surfaceSession?.requestResize(configuration) },
      onFrame = { bitmap, pixelSize, editorRevision, frameKey ->
        surfaceSession?.let { session ->
          editor.deliverFrame(
            session = session,
            bitmap = bitmap,
            pixelSize = pixelSize,
            editorRevision = editorRevision,
            frameKey = frameKey,
          )
        }
      },
      onFrameUnavailable = { frameKey ->
        surfaceSession?.let { session -> editor.surfaceUnavailable(session, frameKey) }
      },
      onTargetUnavailable = onTargetUnavailable@{ frameKey ->
          if (page !in editor.surfacePageRequirements) return@onTargetUnavailable
          val replaceOrFail = {
            if (replacingUnavailableTarget) {
              onFailure(EditorSurfaceUnavailableException(page))
            } else {
              replacingUnavailableTarget = true
              instance += 1
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
