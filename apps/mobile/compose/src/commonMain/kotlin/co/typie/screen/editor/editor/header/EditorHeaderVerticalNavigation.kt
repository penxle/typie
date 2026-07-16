package co.typie.screen.editor.editor.header

import androidx.compose.foundation.gestures.awaitEachGesture
import androidx.compose.foundation.gestures.awaitFirstDown
import androidx.compose.runtime.withFrameNanos
import androidx.compose.ui.Modifier
import androidx.compose.ui.input.key.Key
import androidx.compose.ui.input.key.KeyEvent
import androidx.compose.ui.input.key.isAltPressed
import androidx.compose.ui.input.key.isCtrlPressed
import androidx.compose.ui.input.key.isMetaPressed
import androidx.compose.ui.input.key.isShiftPressed
import androidx.compose.ui.input.key.key
import androidx.compose.ui.input.pointer.pointerInput
import androidx.compose.ui.text.TextLayoutResult
import androidx.compose.ui.text.input.TextFieldValue
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.launch

internal data class HeaderVerticalExitProbe(
  val token: Long,
  val value: TextFieldValue,
  val visualLine: Int,
)

internal class HeaderVerticalNavigationState(
  private val scope: CoroutineScope,
  private val currentValue: () -> TextFieldValue,
  private val currentEnabled: () -> Boolean,
) {
  private var focused = false
  private var latestToken = 0L
  private var latestLayout: TextLayoutResult? = null

  fun onFocusChanged(isFocused: Boolean) {
    if (focused != isFocused) {
      focused = isFocused
      invalidate()
    }
  }

  fun onTextLayout(layout: TextLayoutResult) {
    val previous = latestLayout
    if (
      previous == null || previous.layoutInput != layout.layoutInput || previous.size != layout.size
    ) {
      latestLayout = layout
    }
  }

  fun invalidate() {
    latestToken += 1
  }

  fun handleVerticalKeyDown(event: KeyEvent, onExitUp: (() -> Unit)?, onExitDown: (() -> Unit)?) {
    val onExit =
      when (event.key) {
        Key.DirectionUp -> onExitUp
        Key.DirectionDown -> onExitDown
        else -> null
      }
    if (
      onExit == null ||
        event.isShiftPressed ||
        event.isAltPressed ||
        event.isMetaPressed ||
        event.isCtrlPressed
    ) {
      return
    }

    val value = currentValue()
    if (
      !currentEnabled() ||
        !focused ||
        value.selection.start != value.selection.end ||
        value.composition != null
    ) {
      return
    }

    val layout = latestLayout ?: return
    val probe =
      HeaderVerticalExitProbe(
        token = latestToken,
        value = value,
        visualLine = layout.visualLineFor(value) ?: return,
      )
    scope.launch {
      withFrameNanos {}
      val valueAfterNativeMove = currentValue()
      val visualLineAfterNativeMove =
        layout.takeIf { latestLayout === it }?.visualLineFor(valueAfterNativeMove)
      if (
        shouldExitHeaderFieldAfterNativeVerticalMove(
          probe = probe,
          currentValue = valueAfterNativeMove,
          focused = focused,
          enabled = currentEnabled(),
          latestToken = latestToken,
          currentVisualLine = visualLineAfterNativeMove,
        )
      ) {
        invalidate()
        onExit()
      }
    }
  }
}

internal fun Modifier.invalidateHeaderVerticalNavigationOnPointerDown(
  state: HeaderVerticalNavigationState
): Modifier =
  pointerInput(state) {
    awaitEachGesture {
      awaitFirstDown(requireUnconsumed = false)
      state.invalidate()
    }
  }

internal fun TextLayoutResult.visualLineFor(value: TextFieldValue): Int? {
  if (layoutInput.text.text != value.text || value.selection.start !in 0..value.text.length) {
    return null
  }

  return getLineForOffset(value.selection.start)
}

internal fun shouldExitHeaderFieldAfterNativeVerticalMove(
  probe: HeaderVerticalExitProbe,
  currentValue: TextFieldValue,
  focused: Boolean,
  enabled: Boolean,
  latestToken: Long,
  currentVisualLine: Int?,
): Boolean =
  enabled &&
    focused &&
    probe.token == latestToken &&
    currentVisualLine != null &&
    probe.visualLine == currentVisualLine &&
    probe.value.text == currentValue.text &&
    probe.value.selection.start == probe.value.selection.end &&
    currentValue.selection.start == currentValue.selection.end &&
    probe.value.composition == null &&
    currentValue.composition == null
