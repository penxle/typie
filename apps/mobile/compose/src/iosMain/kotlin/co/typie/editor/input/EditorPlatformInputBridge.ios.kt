@file:OptIn(ExperimentalForeignApi::class, ExperimentalTime::class)

package co.typie.editor.input

import androidx.compose.runtime.withFrameNanos
import androidx.compose.ui.input.key.Key
import androidx.compose.ui.input.key.KeyEvent
import androidx.compose.ui.input.key.isAltPressed
import androidx.compose.ui.input.key.isCtrlPressed
import androidx.compose.ui.input.key.isMetaPressed
import androidx.compose.ui.input.key.isShiftPressed
import androidx.compose.ui.input.key.key
import androidx.compose.ui.text.input.EditCommand
import co.typie.editor.EditorState
import co.typie.editor.EditorViewportTransform
import co.typie.editor.ffi.CursorMetrics
import co.typie.editor.ffi.Message
import kotlin.time.Clock
import kotlin.time.ExperimentalTime
import kotlinx.cinterop.ExperimentalForeignApi
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.CoroutineStart
import kotlinx.coroutines.launch
import swiftPMImport.co.typie.compose.EditorFloatingCursorBridge
import swiftPMImport.co.typie.compose.EditorTextInputTraitsBridge

internal actual class EditorPlatformInputBridge actual constructor() {
  private val physicalKeyGate = EditorPhysicalKeyFrameGate()
  private val selectionIntentTracker = EditorSelectionInputIntentTracker()
  private val floatingCursorSession = EditorFloatingCursorSession()

  actual fun reset() {
    physicalKeyGate.reset()
    selectionIntentTracker.reset()
    floatingCursorSession.end()
  }

  actual fun onPreKeyEvent(
    event: KeyEvent,
    editorState: () -> EditorState,
    inputCoroutineScope: CoroutineScope,
    bindingMessages: suspend () -> List<Message>,
    commit: suspend (List<Message>) -> EditorState?,
  ): Boolean {
    val stroke = event.toPhysicalKeyStroke()
    if (!physicalKeyGate.accept(stroke)) {
      return true
    }
    inputCoroutineScope.launch {
      withFrameNanos {}
      physicalKeyGate.clear(stroke)
    }

    val preState = editorState()
    inputCoroutineScope.launch(start = CoroutineStart.UNDISPATCHED) {
      val messages = bindingMessages()
      if (messages.isEmpty()) {
        selectionIntentTracker.reset()
        return@launch
      }
      val dispatchToken =
        selectionIntentTracker.recordAppOwnedDispatch(
          messages = messages,
          preState = preState,
          nowMillis = nowMillis(),
        )
      val postState =
        commit(messages)
          ?: run {
            dispatchToken?.let(selectionIntentTracker::cancelAppOwnedDispatch)
            return@launch
          }
      if (dispatchToken != null) {
        selectionIntentTracker.recordAppOwnedCommit(
          token = dispatchToken,
          messages = messages,
          preState = preState,
          postState = postState,
          nowMillis = nowMillis(),
        )
      }
    }

    return true
  }

  actual fun shouldConsumeKeyEvent(event: KeyEvent): Boolean = true

  actual fun interceptEditCommands(
    commands: List<EditCommand>,
    state: EditorState,
  ): List<Message>? {
    return when (
      val decision =
        selectionIntentTracker.classifyNativeSelectionCommands(
          commands = commands,
          state = state,
          nowMillis = nowMillis(),
        )
    ) {
      EditorSelectionInputDecision.DropNativeSelectionCommand -> emptyList()
      is EditorSelectionInputDecision.ReplayNativeCommandAsAppOwnedNavigation -> decision.messages
      null -> null
    }
  }

  actual fun onImeMessagesCommitted(
    messages: List<Message>,
    preState: EditorState,
    postState: EditorState,
  ) {
    selectionIntentTracker.recordImeMessagesCommitted(
      messages = messages,
      preState = preState,
      postState = postState,
      nowMillis = nowMillis(),
    )
  }

  actual fun installSessionEffects(
    cursor: () -> CursorMetrics?,
    viewportTransform: () -> EditorViewportTransform,
    dispatch: (List<Message>) -> Unit,
  ): () -> Unit {
    val traitsGeneration = EditorTextInputTraitsBridge.install()
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
      EditorTextInputTraitsBridge.uninstallWithGeneration(traitsGeneration)
    }
  }
}

private fun nowMillis(): Long = Clock.System.now().toEpochMilliseconds()

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
