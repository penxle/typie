package co.typie.screen.editor.editor.viewport

import androidx.compose.foundation.gestures.awaitEachGesture
import androidx.compose.foundation.gestures.awaitFirstDown
import androidx.compose.foundation.gestures.calculateCentroid
import androidx.compose.foundation.gestures.calculateCentroidSize
import androidx.compose.foundation.gestures.calculateZoom
import androidx.compose.runtime.Composable
import androidx.compose.runtime.rememberUpdatedState
import androidx.compose.ui.Modifier
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.hapticfeedback.HapticFeedbackType
import androidx.compose.ui.input.pointer.pointerInput
import androidx.compose.ui.input.pointer.positionChanged
import androidx.compose.ui.platform.LocalHapticFeedback
import co.typie.editor.EditorViewportAnchor
import co.typie.editor.EditorZoomController
import co.typie.editor.EditorZoomSnapKey
import co.typie.editor.body.EditorDocumentLayoutSpec
import co.typie.editor.ffi.Size as PageSize
import co.typie.editor.runtime.EditorUiState
import co.typie.editor.viewport.syncViewportToZoomAnchor
import co.typie.screen.editor.editor.state.EditorScreenState
import kotlin.math.abs

@Composable
internal fun rememberEditorTouchPinchZoomModifier(
  state: EditorScreenState,
  layoutSpec: EditorDocumentLayoutSpec.Paginated,
  zoomController: EditorZoomController,
  uiState: EditorUiState,
  pageSizes: List<PageSize>,
  density: Float,
): Modifier {
  val haptic = LocalHapticFeedback.current
  val currentHaptic = rememberUpdatedState(haptic)

  return Modifier.pointerInput(state, layoutSpec, zoomController, uiState, pageSizes, density) {
    var transformActive = false
    var anchor: EditorViewportAnchor? = null
    var rawZoom = 1f

    fun beginPinchZoom(nextAnchor: EditorViewportAnchor) {
      if (anchor != null) {
        return
      }
      if (!transformActive) {
        state.viewportState.beginTransform()
        transformActive = true
      }
      anchor = nextAnchor
      rawZoom = zoomController.displayZoom
    }

    fun finishPinchZoom() {
      if (anchor != null) {
        zoomController.commitRenderZoom()
        anchor = null
      }
      if (transformActive) {
        state.viewportState.endTransform()
        transformActive = false
      }
      rawZoom = 1f
    }

    try {
      awaitEachGesture {
        awaitFirstDown(requireUnconsumed = false)
        var cumulativeZoom = 1f
        var pastTouchSlop = false
        var event = awaitPointerEvent()

        while (true) {
          val pointerCount = event.changes.count { it.pressed }
          val canceled = event.changes.any { it.isConsumed }
          var consumed = false

          if (canceled || pointerCount < 2) {
            finishPinchZoom()
            cumulativeZoom = 1f
            pastTouchSlop = false
          } else {
            val zoomChange = event.calculateZoom()
            if (zoomChange.isFinite() && zoomChange > 0f) {
              if (!pastTouchSlop) {
                cumulativeZoom *= zoomChange
                val zoomMotion =
                  abs(1 - cumulativeZoom) * event.calculateCentroidSize(useCurrent = false)
                pastTouchSlop = zoomMotion > viewConfiguration.touchSlop
              }

              val centroid = event.calculateCentroid(useCurrent = false)
              if (pastTouchSlop && centroid != Offset.Unspecified) {
                consumed = true
                val focalInEditor =
                  uiState.containerToEditorLocal(x = centroid.x / density, y = centroid.y / density)

                if (focalInEditor != null && state.viewport.width > 0f) {
                  val currentAnchor = anchor
                  if (currentAnchor == null) {
                    val nextAnchor =
                      uiState
                        .resolveViewportTransform(pageSizes = pageSizes)
                        .resolveAnchor(focalX = focalInEditor.x, focalY = focalInEditor.y)
                    if (nextAnchor != null) {
                      beginPinchZoom(nextAnchor)
                    }
                  } else {
                    rawZoom *= zoomChange
                    val previousZoom = zoomController.displayZoom
                    val previousSnap = zoomController.resolveSnapKey(previousZoom)
                    val changed =
                      zoomController.setDisplayZoom(
                        zoom = rawZoom,
                        layoutSpec = layoutSpec,
                        viewportWidth = state.viewport.width,
                      )

                    if (changed) {
                      val nextZoom = zoomController.displayZoom
                      maybeSendZoomSnapHaptic(
                        previousSnap = previousSnap,
                        nextSnap = zoomController.resolveSnapKey(nextZoom),
                        haptic = {
                          currentHaptic.value.performHapticFeedback(HapticFeedbackType.SegmentTick)
                        },
                      )
                      syncViewportToZoomAnchor(
                        viewportState = state.viewportState,
                        pageSizes = pageSizes,
                        anchor = currentAnchor,
                        focalX = focalInEditor.x,
                        focalY = focalInEditor.y,
                        displayZoom = nextZoom,
                        isUserScroll = true,
                      )
                    }
                  }
                }
              }
            }
          }

          if (consumed) {
            event.changes.forEach { change ->
              if (change.positionChanged()) {
                change.consume()
              }
            }
          }
          if (!event.changes.any { it.pressed }) {
            break
          }
          event = awaitPointerEvent()
        }

        finishPinchZoom()
      }
    } finally {
      finishPinchZoom()
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
