package co.typie.editor.input

import androidx.compose.ui.input.key.Key
import androidx.compose.ui.input.key.KeyEvent
import androidx.compose.ui.platform.PlatformTextInputSessionScope
import co.typie.editor.EditorState
import co.typie.editor.EditorViewportTransform
import co.typie.editor.KeyModifier
import co.typie.editor.ffi.CursorMetrics
import co.typie.editor.ffi.Message
import co.typie.editor.ffi.NavigationOp
import kotlinx.coroutines.CoroutineScope

internal expect class EditorPlatformInputBridge() {
  fun reset()

  fun setInputSessionActive(active: Boolean)

  fun bindInputSession(session: PlatformTextInputSessionScope)

  fun resetPlatformInputBeforeBindingDispatch()

  fun takeDocumentNavigation(): NavigationOp.Move?

  fun onPreKeyEvent(
    event: KeyEvent,
    inputCoroutineScope: CoroutineScope,
    onAccepted: () -> Unit,
  ): Boolean

  fun shouldConsumeKeyEvent(event: KeyEvent): Boolean

  fun onImeMessagesApplied(messages: List<Message>, preState: EditorState, postState: EditorState)

  fun installSessionEffects(
    cursor: () -> CursorMetrics?,
    viewportTransform: () -> EditorViewportTransform,
    dispatch: (List<Message>) -> Unit,
    dispatchBindingOnUnmatchedKeyUp: (Key, Set<KeyModifier>) -> Boolean,
  ): () -> Unit
}
