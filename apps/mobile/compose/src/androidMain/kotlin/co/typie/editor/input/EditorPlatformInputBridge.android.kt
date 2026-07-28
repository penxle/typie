// cspell:ignore Gboard reentrantly

package co.typie.editor.input

import android.content.Context
import android.view.View
import android.view.inputmethod.InputMethodManager
import androidx.compose.ui.input.key.Key
import androidx.compose.ui.input.key.KeyEvent
import androidx.compose.ui.platform.PlatformTextInputSessionScope
import androidx.compose.ui.text.input.EditCommand
import co.typie.editor.EditorState
import co.typie.editor.EditorViewportTransform
import co.typie.editor.KeyModifier
import co.typie.editor.ffi.CursorMetrics
import co.typie.editor.ffi.Message
import java.lang.ref.WeakReference
import kotlinx.coroutines.CoroutineScope

internal actual class EditorPlatformInputBridge actual constructor() {
  private var inputView = WeakReference<View>(null)

  actual fun reset() = Unit

  actual fun setInputSessionActive(active: Boolean) {
    if (!active) inputView.clear()
  }

  actual fun bindInputSession(session: PlatformTextInputSessionScope) {
    inputView = WeakReference(session.view)
  }

  actual fun resetPlatformInputBeforeBindingDispatch() {
    val view = inputView.get() ?: return

    // Gboard's Korean physical-keyboard path can retain a private composing syllable while
    // exposing every update as commitText plus selection replacement. In that path the editor
    // correctly has no composing range, so updateSelection cannot express the remaining state.
    // Reset only at bindings that explicitly require a composition commit.
    //
    // Post until the current InputConnection key dispatch returns so the connection is not
    // invalidated reentrantly.
    view.post {
      if (inputView.get() !== view) return@post
      val inputMethodManager =
        view.context.getSystemService(Context.INPUT_METHOD_SERVICE) as? InputMethodManager
      inputMethodManager?.restartInput(view)
    }
  }

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
  ) = Unit

  actual fun installSessionEffects(
    cursor: () -> CursorMetrics?,
    viewportTransform: () -> EditorViewportTransform,
    dispatch: (List<Message>) -> Unit,
    dispatchBindingOnUnmatchedKeyUp: (Key, Set<KeyModifier>) -> Boolean,
  ): () -> Unit = {}
}
