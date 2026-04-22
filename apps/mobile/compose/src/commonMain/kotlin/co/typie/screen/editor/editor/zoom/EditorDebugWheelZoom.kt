package co.typie.screen.editor.editor.zoom

import androidx.compose.runtime.Composable
import androidx.compose.ui.Modifier
import androidx.compose.ui.input.pointer.PointerEventPass
import androidx.compose.ui.input.pointer.PointerEventType
import androidx.compose.ui.input.pointer.isCtrlPressed
import androidx.compose.ui.input.pointer.isMetaPressed
import androidx.compose.ui.input.pointer.pointerInput
import androidx.compose.ui.platform.LocalDensity
import co.typie.editor.EditorZoomController
import co.typie.editor.body.EditorDocumentLayoutSpec
import co.typie.editor.clampDocumentZoom
import co.typie.editor.computePaginatedZoomBounds
import co.typie.editor.ffi.Size as PageSize
import co.typie.editor.runtime.EditorUiState
import co.typie.screen.editor.editor.state.EditorScreenState
import kotlin.math.abs
import kotlin.math.exp

private const val WheelZoomDivisor = 240f
private const val WheelBurstGapMs = 56L
private const val WheelTailDeltaPx = 0.8f
private const val WheelTailStreakToReset = 3
private const val WheelModeSwitchMinDeltaPx = 1.5f
private const val WheelDeltaNormalizationScale = 10f

@Composable
internal fun rememberEditorDebugWheelZoomModifier(
  enabled: Boolean,
  state: EditorScreenState,
  layoutSpec: EditorDocumentLayoutSpec,
  zoomController: EditorZoomController,
  uiState: EditorUiState,
  pageSizes: List<PageSize>,
): Modifier {
  if (!enabled) {
    return Modifier
  }

  val density = LocalDensity.current.density

  return Modifier.pointerInput(state, layoutSpec, zoomController, uiState, pageSizes, density) {
    val paginatedLayout = layoutSpec as? EditorDocumentLayoutSpec.Paginated ?: return@pointerInput
    if (density <= 0f) {
      return@pointerInput
    }

    var wheelLastEventTs: Long? = null
    var wheelLowDeltaStreak = 0
    var wheelRawZoom: Float? = null
    var wheelZoomSessionActive = false
    fun finishWheelZoomSession() {
      if (wheelZoomSessionActive) {
        zoomController.commitRenderZoom()
      }
      wheelRawZoom = null
      wheelLowDeltaStreak = 0
      wheelZoomSessionActive = false
    }

    while (true) {
      val event = awaitPointerEventScope { awaitPointerEvent(PointerEventPass.Initial) }
      if (event.type != PointerEventType.Scroll) {
        continue
      }

      val hasZoomModifier =
        event.keyboardModifiers.isMetaPressed || event.keyboardModifiers.isCtrlPressed
      val change = event.changes.firstOrNull() ?: continue
      val dominantDelta =
        if (abs(change.scrollDelta.y) >= abs(change.scrollDelta.x)) {
          change.scrollDelta.y
        } else {
          change.scrollDelta.x
        }
      if (
        !hasZoomModifier ||
          !dominantDelta.isFinite() ||
          dominantDelta == 0f ||
          state.viewport.width <= 0f
      ) {
        continue
      }

      val normalizedDelta = normalizeWheelDelta(dominantDelta)
      val deltaMagnitude = abs(normalizedDelta)
      val elapsedSinceLastEvent =
        wheelLastEventTs?.let { change.uptimeMillis - it } ?: Long.MAX_VALUE
      wheelLastEventTs = change.uptimeMillis

      if (elapsedSinceLastEvent > WheelBurstGapMs) {
        finishWheelZoomSession()
      }

      if (deltaMagnitude <= WheelTailDeltaPx) {
        wheelLowDeltaStreak += 1
        if (wheelLowDeltaStreak >= WheelTailStreakToReset) {
          finishWheelZoomSession()
          continue
        }
      } else {
        wheelLowDeltaStreak = 0
      }

      if (!wheelZoomSessionActive) {
        if (deltaMagnitude < WheelModeSwitchMinDeltaPx) {
          continue
        }
        wheelZoomSessionActive = true
      }

      event.changes.forEach { it.consume() }

      val wheelBaseZoom = wheelRawZoom ?: zoomController.displayZoom
      val nextRawZoom =
        clampDocumentZoom(
          zoom = wheelBaseZoom * exp((-normalizedDelta / WheelZoomDivisor).toDouble()).toFloat(),
          bounds = computePaginatedZoomBounds(paginatedLayout.pageWidth),
        )
      wheelRawZoom = nextRawZoom

      val displayZoomBeforeChange = zoomController.displayZoom
      val changed =
        zoomController.setDisplayZoom(
          zoom = nextRawZoom,
          layoutSpec = paginatedLayout,
          viewportWidth = state.viewport.width,
        )
      if (!changed) {
        continue
      }

      val focalInEditor =
        uiState.containerToEditorLocal(
          x = change.position.x / density,
          y = change.position.y / density,
        )
      if (focalInEditor == null) {
        continue
      }

      val anchor =
        uiState
          .resolveViewportTransform(pageSizes = pageSizes)
          .copy(displayZoom = displayZoomBeforeChange)
          .resolveAnchor(focalX = focalInEditor.x, focalY = focalInEditor.y) ?: continue

      syncViewportToZoomAnchor(
        state = state,
        pageSizes = pageSizes,
        anchor = anchor,
        focalX = focalInEditor.x,
        focalY = focalInEditor.y,
        displayZoom = zoomController.displayZoom,
        density = density,
      )
    }
  }
}

private fun normalizeWheelDelta(delta: Float): Float {
  if (!delta.isFinite()) {
    return 0f
  }

  // Compose Desktop scrollDelta는 웹의 wheel delta처럼 px가 아니라 플랫폼별 tick 단위다.
  // 디버그 wheel zoom도 기존 감각을 유지하도록 같은 보정값을 적용한다.
  return delta * WheelDeltaNormalizationScale
}
