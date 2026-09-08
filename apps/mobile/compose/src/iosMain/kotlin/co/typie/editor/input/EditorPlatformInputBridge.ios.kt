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
import androidx.compose.ui.platform.PlatformTextInputSessionScope
import co.typie.editor.EditorState
import co.typie.editor.EditorViewportTransform
import co.typie.editor.KeyModifier
import co.typie.editor.ffi.CursorMetrics
import co.typie.editor.ffi.Direction
import co.typie.editor.ffi.Message
import co.typie.editor.ffi.Movement
import co.typie.editor.ffi.NavigationOp
import kotlinx.cinterop.ExperimentalForeignApi
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.launch
import swiftPMImport.co.typie.compose.EditorFloatingCursorBridge
import swiftPMImport.co.typie.compose.EditorKeyboardBridge
import swiftPMImport.co.typie.compose.EditorTextInputBridge

internal actual class EditorPlatformInputBridge actual constructor() {
  private val physicalKeyGate = EditorPhysicalKeyFrameGate()
  private val floatingCursorSession = EditorFloatingCursorSession()

  actual fun reset() {
    physicalKeyGate.reset()
    floatingCursorSession.end()
  }

  actual fun setInputSessionActive(active: Boolean) = Unit

  actual fun bindInputSession(session: PlatformTextInputSessionScope) = Unit

  actual fun resetPlatformInputBeforeBindingDispatch() {
    EditorKeyboardBridge.endInputMethodComposition()
  }

  actual fun takeDocumentNavigation(): NavigationOp.Move? {
    var movement: NavigationOp.Move? = null
    EditorTextInputBridge.takeDocumentNavigation { backward, extending ->
      movement =
        NavigationOp.Move(
          Movement.Grapheme(if (backward) Direction.Backward else Direction.Forward),
          extending,
        )
    }
    return movement
  }

  actual fun onPreKeyEvent(
    event: KeyEvent,
    inputCoroutineScope: CoroutineScope,
    onAccepted: () -> Unit,
  ): Boolean {
    val stroke = event.toPhysicalKeyStroke()
    if (!physicalKeyGate.accept(stroke)) {
      return true
    }
    inputCoroutineScope.launch {
      withFrameNanos {}
      physicalKeyGate.clear(stroke)
    }

    onAccepted()

    return true
  }

  actual fun shouldConsumeKeyEvent(event: KeyEvent): Boolean = true

  actual fun onImeMessagesApplied(
    messages: List<Message>,
    preState: EditorState,
    postState: EditorState,
  ) = Unit

  actual fun installSessionEffects(
    cursor: () -> CursorMetrics?,
    viewportTransform: () -> EditorViewportTransform,
    dispatch: (List<Message>) -> Unit,
    dispatchBindingOnUnmatchedKeyUp: (Key, Set<KeyModifier>) -> Boolean,
  ): () -> Unit {
    val textInputGeneration = EditorTextInputBridge.install()
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
      EditorTextInputBridge.uninstallWithGeneration(textInputGeneration)
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
