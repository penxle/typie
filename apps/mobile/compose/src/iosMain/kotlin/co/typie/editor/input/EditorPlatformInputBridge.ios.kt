@file:OptIn(ExperimentalForeignApi::class)

package co.typie.editor.input

import androidx.compose.runtime.withFrameNanos
import androidx.compose.ui.input.key.Key
import androidx.compose.ui.input.key.KeyEvent
import androidx.compose.ui.input.key.isAltPressed
import androidx.compose.ui.input.key.isCtrlPressed
import androidx.compose.ui.input.key.isMetaPressed
import androidx.compose.ui.input.key.isShiftPressed
import androidx.compose.ui.input.key.key
import co.typie.editor.EditorViewportTransform
import co.typie.editor.ffi.CursorMetrics
import co.typie.editor.ffi.ImeRange
import co.typie.editor.ffi.Message
import kotlinx.cinterop.ExperimentalForeignApi
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.delay
import kotlinx.coroutines.launch
import swiftPMImport.co.typie.compose.EditorFloatingCursorBridge

private const val SelectionEchoTimeoutMillis = 150L

internal actual class EditorPlatformInputBridge actual constructor() {
  private val physicalKeyGate = EditorPhysicalKeyFrameGate()
  private val selectionEchoTracker = EditorInputSelectionEchoTracker()
  private val floatingCursorSession = EditorFloatingCursorSession()

  actual fun reset() {
    physicalKeyGate.reset()
    selectionEchoTracker.reset()
    floatingCursorSession.end()
  }

  actual fun onPreKeyEvent(
    event: KeyEvent,
    selection: ImeRange?,
    inputCoroutineScope: CoroutineScope,
    dispatch: () -> Unit,
  ): Boolean {
    val stroke = event.toPhysicalKeyStroke()
    if (!physicalKeyGate.accept(stroke)) {
      return true
    }
    inputCoroutineScope.launch {
      withFrameNanos {}
      physicalKeyGate.clear(stroke)
    }

    dispatch()

    event.toSelectionEchoDirection()?.let { direction ->
      val echo =
        selectionEchoTracker.expect(
          direction = direction,
          selection = selection,
          extend = event.isShiftPressed,
        )
      inputCoroutineScope.launch {
        delay(SelectionEchoTimeoutMillis)
        selectionEchoTracker.expire(echo)
      }
    }

    return true
  }

  actual fun shouldConsumeKeyEvent(event: KeyEvent): Boolean = true

  actual fun interceptImeMessages(messages: List<Message>): List<Message> =
    if (selectionEchoTracker.consumeIfEcho(messages)) emptyList() else messages

  actual fun installSessionEffects(
    cursor: () -> CursorMetrics?,
    viewportTransform: () -> EditorViewportTransform,
    dispatch: (List<Message>) -> Unit,
  ): () -> Unit {
    val uninstall =
      installFloatingCursorBridge(
        onBegin = { floatingCursorSession.begin(cursor()) },
        onUpdate = { dx, dy ->
          floatingCursorSession
            .update(dx = dx, dy = dy, transform = viewportTransform())
            ?.let(dispatch)
        },
        onEnd = { floatingCursorSession.end() },
      )

    return {
      uninstall()
      floatingCursorSession.end()
    }
  }
}

private class EditorPhysicalKeyFrameGate {
  private val pending = mutableSetOf<PhysicalKeyStroke>()

  fun accept(stroke: PhysicalKeyStroke): Boolean = pending.add(stroke)

  fun clear(stroke: PhysicalKeyStroke) {
    pending.remove(stroke)
  }

  fun reset() {
    pending.clear()
  }
}

private data class PhysicalKeyStroke(
  val key: Key,
  val shift: Boolean,
  val meta: Boolean,
  val ctrl: Boolean,
  val alt: Boolean,
)

private fun KeyEvent.toPhysicalKeyStroke(): PhysicalKeyStroke =
  PhysicalKeyStroke(
    key = key,
    shift = isShiftPressed,
    meta = isMetaPressed,
    ctrl = isCtrlPressed,
    alt = isAltPressed,
  )

private fun KeyEvent.toSelectionEchoDirection(): EditorInputSelectionEchoDirection? =
  when (key) {
    Key.DirectionLeft -> EditorInputSelectionEchoDirection.Backward
    Key.DirectionRight -> EditorInputSelectionEchoDirection.Forward
    Key.DirectionUp,
    Key.DirectionDown -> EditorInputSelectionEchoDirection.Vertical

    else -> null
  }

private fun installFloatingCursorBridge(
  onBegin: () -> Unit,
  onUpdate: (dx: Float, dy: Float) -> Unit,
  onEnd: () -> Unit,
): () -> Unit {
  EditorFloatingCursorBridge.onBegin = onBegin
  EditorFloatingCursorBridge.onUpdate = { dx, dy -> onUpdate(dx.toFloat(), dy.toFloat()) }
  EditorFloatingCursorBridge.onEnd = onEnd
  val generation = EditorFloatingCursorBridge.install()

  return { EditorFloatingCursorBridge.clearHandlersForInstallWithGeneration(generation) }
}
