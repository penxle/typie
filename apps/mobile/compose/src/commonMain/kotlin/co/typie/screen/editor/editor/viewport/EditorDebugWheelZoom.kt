package co.typie.screen.editor.editor.viewport

import androidx.compose.runtime.Composable
import androidx.compose.ui.Modifier
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.input.pointer.PointerEventPass
import androidx.compose.ui.input.pointer.PointerEventType
import androidx.compose.ui.input.pointer.isCtrlPressed
import androidx.compose.ui.input.pointer.isMetaPressed
import androidx.compose.ui.input.pointer.pointerInput
import co.typie.editor.viewport.normalizeEditorViewportWheelZoomDelta
import co.typie.screen.editor.editor.state.EditorScreenState
import kotlin.math.abs
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Job
import kotlinx.coroutines.coroutineScope
import kotlinx.coroutines.delay
import kotlinx.coroutines.launch

private const val WheelBurstGapMs = 56L
private const val WheelTailDeltaPx = 0.8f
private const val WheelTailStreakToReset = 3
private const val WheelModeSwitchMinDeltaPx = 1.5f

@Composable
internal fun rememberEditorDebugWheelZoomModifier(
  state: EditorScreenState,
  onZoomSessionStart: () -> Boolean,
  onZoom: (focalPosition: Offset, normalizedDelta: Float) -> Boolean,
  onZoomSessionEnd: () -> Unit,
): Modifier {
  return Modifier.pointerInput(state) {
    coroutineScope {
      var wheelLastEventTs: Long? = null
      var wheelLowDeltaStreak = 0
      val wheelZoomSession =
        EditorDebugWheelZoomSession(
          scope = this,
          timeoutMillis = WheelBurstGapMs,
          onSessionEnd = {
            wheelLowDeltaStreak = 0
            onZoomSessionEnd()
          },
        )

      fun finishWheelZoomSession() {
        wheelLowDeltaStreak = 0
        wheelZoomSession.finish()
      }

      try {
        while (true) {
          val event =
            this@pointerInput.awaitPointerEventScope { awaitPointerEvent(PointerEventPass.Initial) }
          if (event.type != PointerEventType.Scroll) {
            finishWheelZoomSession()
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
          if (!hasZoomModifier) {
            finishWheelZoomSession()
            continue
          }
          if (!dominantDelta.isFinite() || dominantDelta == 0f || state.viewport.width <= 0f) {
            continue
          }

          val normalizedDelta = normalizeEditorViewportWheelZoomDelta(dominantDelta)
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

          if (!wheelZoomSession.active) {
            if (deltaMagnitude < WheelModeSwitchMinDeltaPx) {
              continue
            }
            if (!onZoomSessionStart()) {
              continue
            }
            wheelZoomSession.beginOrKeepAlive()
          }

          if (!onZoom(change.position, normalizedDelta)) {
            finishWheelZoomSession()
            continue
          }
          wheelZoomSession.beginOrKeepAlive()
          event.changes.forEach { it.consume() }
        }
      } finally {
        finishWheelZoomSession()
      }
    }
  }
}

internal class EditorDebugWheelZoomSession(
  private val scope: CoroutineScope,
  private val timeoutMillis: Long,
  private val onSessionEnd: () -> Unit,
) {
  private var timeoutJob: Job? = null

  var active: Boolean = false
    private set

  fun beginOrKeepAlive() {
    active = true
    timeoutJob?.cancel()
    timeoutJob = scope.launch {
      delay(timeoutMillis)
      finishFromTimeout()
    }
  }

  fun finish() {
    val wasActive = active
    active = false
    timeoutJob?.cancel()
    timeoutJob = null
    if (wasActive) {
      onSessionEnd()
    }
  }

  private fun finishFromTimeout() {
    val wasActive = active
    active = false
    timeoutJob = null
    if (wasActive) {
      onSessionEnd()
    }
  }
}
