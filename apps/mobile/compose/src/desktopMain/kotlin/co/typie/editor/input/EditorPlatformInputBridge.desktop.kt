package co.typie.editor.input

import androidx.compose.ui.input.key.Key
import androidx.compose.ui.input.key.KeyEvent
import androidx.compose.ui.platform.PlatformTextInputSessionScope
import androidx.compose.ui.text.input.EditCommand
import co.typie.editor.EditorState
import co.typie.editor.EditorViewportTransform
import co.typie.editor.KeyModifier
import co.typie.editor.ffi.CursorMetrics
import co.typie.editor.ffi.FlatImeOp
import co.typie.editor.ffi.Message
import java.awt.KeyEventDispatcher
import java.awt.KeyboardFocusManager
import java.awt.event.KeyEvent as AwtKeyEvent
import kotlinx.coroutines.CoroutineScope

internal actual class EditorPlatformInputBridge actual constructor() {
  private val pressedKeys = mutableSetOf<DesktopPhysicalKey>()
  private var observedModifiers = emptySet<KeyModifier>()
  private var pendingRecoveryModifiers: Set<KeyModifier>? = null
  private var sessionActive = false
  private var sessionEffectsGeneration = 0

  // Compose may reinstall an active text-input session while a physical key stroke
  // is still in flight. Preserve that tracking until the input session actually ends.
  actual fun reset() = Unit

  actual fun setInputSessionActive(active: Boolean) {
    sessionActive = active
    if (!active) {
      sessionEffectsGeneration += 1
      pressedKeys.clear()
      observedModifiers = emptySet()
      pendingRecoveryModifiers = null
    }
  }

  actual fun bindInputSession(session: PlatformTextInputSessionScope) = Unit

  actual fun resetPlatformInputBeforeBindingDispatch() = Unit

  actual fun onPreKeyEvent(
    event: KeyEvent,
    inputCoroutineScope: CoroutineScope,
    onAccepted: () -> Unit,
  ): Boolean = false

  actual suspend fun dispatchAppOwnedKeyMessages(
    messages: List<Message>,
    preState: EditorState,
    dispatch: suspend () -> EditorState?,
  ) {
    dispatch()
  }

  actual fun shouldConsumeKeyEvent(event: KeyEvent): Boolean = false

  actual fun interceptEditCommands(
    commands: List<EditCommand>,
    state: EditorState,
  ): List<Message>? = null

  actual fun onImeMessagesApplied(
    messages: List<Message>,
    preState: EditorState,
    postState: EditorState,
  ) {
    if (!sessionActive) return

    val committedComposition =
      preState.ime?.composing != null &&
        postState.ime?.composing == null &&
        messages.any { message ->
          message is Message.TextInput && message.ops.any { it == FlatImeOp.CommitAsIs }
        }
    pendingRecoveryModifiers =
      if (committedComposition) {
        observedModifiers
      } else {
        null
      }
  }

  actual fun installSessionEffects(
    cursor: () -> CursorMetrics?,
    viewportTransform: () -> EditorViewportTransform,
    dispatch: (List<Message>) -> Unit,
    dispatchBindingOnUnmatchedKeyUp: (Key, Set<KeyModifier>) -> Boolean,
  ): () -> Unit {
    val focusManager = KeyboardFocusManager.getCurrentKeyboardFocusManager()
    val generation = ++sessionEffectsGeneration
    val keyEventDispatcher = KeyEventDispatcher { event ->
      if (!sessionActive || generation != sessionEffectsGeneration) {
        return@KeyEventDispatcher false
      }

      val physicalKey = event.toDesktopPhysicalKey()
      when (event.id) {
        AwtKeyEvent.KEY_PRESSED -> {
          pendingRecoveryModifiers = null
          pressedKeys += physicalKey
          observedModifiers = event.toKeyModifiers()
          false
        }
        AwtKeyEvent.KEY_RELEASED -> {
          observedModifiers = event.toKeyModifiers()
          if (pressedKeys.remove(physicalKey)) {
            false
          } else {
            val recoveryModifiers = pendingRecoveryModifiers ?: return@KeyEventDispatcher false
            pendingRecoveryModifiers = null
            // JVM IMEs can commit composition while consuming the triggering KeyDown, then
            // still deliver KeyUp through AWT. Recover only after that commit was observed,
            // using its modifier snapshot so modifier-first release order remains valid.
            dispatchBindingOnUnmatchedKeyUp(
              Key(physicalKey.code, physicalKey.location),
              recoveryModifiers,
            )
          }
        }
        else -> false
      }
    }

    focusManager.addKeyEventDispatcher(keyEventDispatcher)
    return { focusManager.removeKeyEventDispatcher(keyEventDispatcher) }
  }
}

private data class DesktopPhysicalKey(val code: Int, val location: Int)

private fun AwtKeyEvent.toDesktopPhysicalKey(): DesktopPhysicalKey =
  DesktopPhysicalKey(
    code = keyCode,
    location =
      if (keyLocation == AwtKeyEvent.KEY_LOCATION_UNKNOWN) {
        AwtKeyEvent.KEY_LOCATION_STANDARD
      } else {
        keyLocation
      },
  )

private fun AwtKeyEvent.toKeyModifiers(): Set<KeyModifier> = buildSet {
  if (isShiftDown) add(KeyModifier.Shift)
  if (isMetaDown) add(KeyModifier.Mod)
  if (isControlDown) add(KeyModifier.Ctrl)
  if (isAltDown) add(KeyModifier.Alt)
}
