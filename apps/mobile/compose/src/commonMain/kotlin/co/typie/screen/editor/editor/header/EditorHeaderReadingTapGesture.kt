package co.typie.screen.editor.editor.header

import androidx.compose.foundation.gestures.awaitEachGesture
import androidx.compose.foundation.gestures.awaitFirstDown
import androidx.compose.ui.Modifier
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.input.pointer.PointerEventPass
import androidx.compose.ui.input.pointer.pointerInput
import co.typie.editor.interaction.gestures.EditorConsecutiveTapMaxIntervalMillis
import kotlinx.coroutines.Job
import kotlinx.coroutines.coroutineScope
import kotlinx.coroutines.delay
import kotlinx.coroutines.launch

internal sealed interface EditorHeaderReadingTapResult {
  data object None : EditorHeaderReadingTapResult

  data object ShowHint : EditorHeaderReadingTapResult

  data class Activate(val offset: Int) : EditorHeaderReadingTapResult
}

internal class EditorHeaderReadingTapTracker(
  private val touchSlopPx: Float,
  private val maxTapDistancePx: Float,
  private val doubleTapToEditEnabled: Boolean,
  private val editingActivationEnabled: Boolean = true,
  private val consecutiveTapMaxIntervalMillis: Long = EditorConsecutiveTapMaxIntervalMillis,
) {
  private data class ActiveTap(val position: Offset, val suppressHint: Boolean)

  private data class PendingTap(
    val position: Offset,
    val confirmationAtMillis: Long,
    val suppressHint: Boolean,
  )

  private var activeTap: ActiveTap? = null
  private var pendingTap: PendingTap? = null

  val pendingConfirmationAtMillis: Long?
    get() = pendingTap?.confirmationAtMillis

  fun onDown(
    position: Offset,
    offset: Int,
    timeMillis: Long,
    suppressHint: Boolean = false,
  ): EditorHeaderReadingTapResult {
    val pending = pendingTap
    pendingTap = null
    val isConsecutiveTap =
      pending != null &&
        timeMillis <= pending.confirmationAtMillis &&
        (position - pending.position).getDistance() <= maxTapDistancePx
    if (editingActivationEnabled && doubleTapToEditEnabled && isConsecutiveTap) {
      activeTap = null
      return EditorHeaderReadingTapResult.Activate(offset)
    }

    activeTap =
      ActiveTap(
        position = position,
        suppressHint = suppressHint || (isConsecutiveTap && !editingActivationEnabled),
      )
    return EditorHeaderReadingTapResult.None
  }

  fun onMove(position: Offset) {
    val active = activeTap ?: return
    if ((position - active.position).getDistance() > touchSlopPx) {
      cancel()
    }
  }

  fun onUp(
    position: Offset,
    offset: Int,
    timeMillis: Long,
    suppressHint: Boolean = false,
  ): EditorHeaderReadingTapResult {
    val active = activeTap ?: return EditorHeaderReadingTapResult.None
    activeTap = null
    if (editingActivationEnabled && !doubleTapToEditEnabled) {
      pendingTap = null
      return EditorHeaderReadingTapResult.Activate(offset)
    }

    pendingTap =
      PendingTap(
        position = position,
        confirmationAtMillis = timeMillis + consecutiveTapMaxIntervalMillis,
        suppressHint = active.suppressHint || suppressHint,
      )
    return EditorHeaderReadingTapResult.None
  }

  fun confirm(timeMillis: Long): EditorHeaderReadingTapResult {
    val pending = pendingTap ?: return EditorHeaderReadingTapResult.None
    if (timeMillis < pending.confirmationAtMillis) {
      return EditorHeaderReadingTapResult.None
    }

    pendingTap = null
    return if (pending.suppressHint) {
      EditorHeaderReadingTapResult.None
    } else {
      EditorHeaderReadingTapResult.ShowHint
    }
  }

  fun onLongPress() {
    cancel()
  }

  fun onModeChanged() {
    cancel()
  }

  fun cancel() {
    activeTap = null
    pendingTap = null
  }
}

internal fun Modifier.observeEditorHeaderReadingTaps(
  enabled: () -> Boolean,
  tracker: EditorHeaderReadingTapTracker,
  currentSelectionIsExpanded: () -> Boolean,
  offsetForPosition: (Offset) -> Int,
  onActivate: (Int) -> Unit,
  onShowHint: () -> Unit,
): Modifier {
  return pointerInput(tracker) {
    coroutineScope {
      var confirmationJob: Job? = null
      try {
        awaitEachGesture {
          val down = awaitFirstDown(requireUnconsumed = false, pass = PointerEventPass.Initial)
          if (down.isConsumed) {
            tracker.cancel()
            return@awaitEachGesture
          }
          if (!enabled()) {
            tracker.cancel()
            return@awaitEachGesture
          }
          confirmationJob?.cancel()
          confirmationJob = null
          val downOffset = offsetForPosition(down.position)
          var activationOwnsPointer = false
          when (
            val result =
              tracker.onDown(
                position = down.position,
                offset = downOffset,
                timeMillis = down.uptimeMillis,
                suppressHint = currentSelectionIsExpanded(),
              )
          ) {
            is EditorHeaderReadingTapResult.Activate -> {
              down.consume()
              onActivate(result.offset)
              activationOwnsPointer = true
            }
            EditorHeaderReadingTapResult.None -> Unit
            EditorHeaderReadingTapResult.ShowHint -> error("A pointer down cannot confirm a hint")
          }

          while (true) {
            val event = awaitPointerEvent(pass = PointerEventPass.Initial)
            val change = event.changes.find { it.id == down.id }
            if (change == null) {
              tracker.cancel()
              return@awaitEachGesture
            }
            if (activationOwnsPointer) {
              change.consume()
              if (!change.pressed) {
                return@awaitEachGesture
              }
              continue
            }
            if (change.isConsumed) {
              tracker.cancel()
              return@awaitEachGesture
            }
            if (event.changes.count { it.pressed } > 1) {
              tracker.cancel()
              return@awaitEachGesture
            }
            if (change.pressed) {
              tracker.onMove(change.position)
              continue
            }

            val elapsedMillis = change.uptimeMillis - down.uptimeMillis
            if (elapsedMillis >= viewConfiguration.longPressTimeoutMillis) {
              tracker.onLongPress()
              return@awaitEachGesture
            }

            when (
              val result =
                tracker.onUp(
                  position = change.position,
                  offset = offsetForPosition(change.position),
                  timeMillis = change.uptimeMillis,
                )
            ) {
              is EditorHeaderReadingTapResult.Activate -> onActivate(result.offset)
              EditorHeaderReadingTapResult.None -> {
                val confirmationAtMillis = tracker.pendingConfirmationAtMillis
                if (confirmationAtMillis != null) {
                  confirmationJob = launch {
                    delay((confirmationAtMillis - change.uptimeMillis).coerceAtLeast(0))
                    if (
                      tracker.confirm(confirmationAtMillis) == EditorHeaderReadingTapResult.ShowHint
                    ) {
                      onShowHint()
                    }
                  }
                }
              }
              EditorHeaderReadingTapResult.ShowHint -> onShowHint()
            }
            return@awaitEachGesture
          }
        }
      } finally {
        confirmationJob?.cancel()
        tracker.cancel()
      }
    }
  }
}
