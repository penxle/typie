package co.typie.screen.editor.editor.zoom

import androidx.compose.foundation.gestures.awaitEachGesture
import androidx.compose.foundation.gestures.awaitFirstDown
import androidx.compose.runtime.Composable
import androidx.compose.runtime.remember
import androidx.compose.ui.Modifier
import androidx.compose.ui.hapticfeedback.HapticFeedbackType
import androidx.compose.ui.input.pointer.PointerEventPass
import androidx.compose.ui.input.pointer.pointerInput
import androidx.compose.ui.platform.LocalHapticFeedback
import androidx.compose.ui.platform.LocalViewConfiguration
import androidx.compose.ui.util.fastMapNotNull
import co.typie.editor.EditorViewportAnchor
import co.typie.editor.EditorZoomController
import co.typie.editor.EditorZoomSnapKey
import co.typie.editor.body.EditorDocumentLayoutSpec
import co.typie.editor.ffi.Size as PageSize
import co.typie.editor.runtime.EditorUiState
import co.typie.screen.editor.editor.state.EditorScreenState

private data class PinchGestureSession(
  val startDistancePx: Float,
  val startZoom: Float,
  val anchor: EditorViewportAnchor,
)

@Composable
internal fun rememberEditorTouchPinchZoomModifier(
  state: EditorScreenState,
  layoutSpec: EditorDocumentLayoutSpec,
  zoomController: EditorZoomController,
  uiState: EditorUiState,
  pageSizes: List<PageSize>,
  density: Float,
): Modifier {
  val haptic = LocalHapticFeedback.current
  val touchSlop = LocalViewConfiguration.current.touchSlop

  return remember(
    state,
    layoutSpec,
    zoomController,
    uiState,
    pageSizes,
    density,
    haptic,
    touchSlop,
  ) {
    Modifier.pointerInput(
      state,
      layoutSpec,
      zoomController,
      uiState,
      pageSizes,
      density,
      touchSlop,
    ) {
      val paginatedLayout = layoutSpec as? EditorDocumentLayoutSpec.Paginated ?: return@pointerInput
      if (density <= 0f) {
        return@pointerInput
      }

      awaitEachGesture {
        awaitFirstDown(requireUnconsumed = false, pass = PointerEventPass.Initial)
        var session: PinchGestureSession? = null
        fun finishPinchSession() {
          if (session == null) {
            return
          }

          zoomController.commitRenderZoom()
          session = null
        }

        while (true) {
          val event = awaitPointerEvent(PointerEventPass.Initial)
          val pressed = event.changes.fastMapNotNull { change -> change.takeIf { it.pressed } }
          if (pressed.isEmpty()) {
            finishPinchSession()
            break
          }

          if (pressed.size < 2) {
            finishPinchSession()
            continue
          }

          val first = pressed[0].position
          val second = pressed[1].position
          val delta = second - first
          val distancePx = delta.getDistance()
          if (!distancePx.isFinite() || distancePx <= touchSlop) {
            continue
          }

          val focal = (first + second) / 2f
          val focalInEditor =
            uiState.containerToEditorLocal(x = focal.x / density, y = focal.y / density)
          if (focalInEditor == null || state.viewport.width <= 0f) {
            continue
          }

          if (session == null) {
            val anchor =
              uiState
                .resolveViewportTransform(pageSizes = pageSizes)
                .resolveAnchor(focalX = focalInEditor.x, focalY = focalInEditor.y) ?: continue
            session =
              PinchGestureSession(
                startDistancePx = distancePx,
                startZoom = zoomController.displayZoom,
                anchor = anchor,
              )
            event.changes.forEach { it.consume() }
            continue
          }

          val currentSession = session ?: continue
          val nextRawZoom = currentSession.startZoom * (distancePx / currentSession.startDistancePx)
          val previousZoom = zoomController.displayZoom
          val previousSnap = zoomController.resolveSnapKey(previousZoom)
          val changed =
            zoomController.setDisplayZoom(
              zoom = nextRawZoom,
              layoutSpec = paginatedLayout,
              viewportWidth = state.viewport.width,
            )
          event.changes.forEach { it.consume() }
          if (!changed) {
            continue
          }

          val nextZoom = zoomController.displayZoom
          maybeSendZoomSnapHaptic(
            previousSnap = previousSnap,
            nextSnap = zoomController.resolveSnapKey(nextZoom),
            haptic = { haptic.performHapticFeedback(HapticFeedbackType.SegmentTick) },
          )

          syncViewportToZoomAnchor(
            state = state,
            pageSizes = pageSizes,
            anchor = currentSession.anchor,
            focalX = focalInEditor.x,
            focalY = focalInEditor.y,
            displayZoom = nextZoom,
            density = density,
          )
        }
      }
    }
  }
}

internal fun maybeSendZoomSnapHaptic(
  previousSnap: EditorZoomSnapKey?,
  nextSnap: EditorZoomSnapKey?,
  haptic: () -> Unit,
) {
  if (nextSnap == null || nextSnap == previousSnap) {
    return
  }

  haptic()
}
