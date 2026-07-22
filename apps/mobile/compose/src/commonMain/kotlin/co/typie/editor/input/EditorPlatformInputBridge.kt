package co.typie.editor.input

import androidx.compose.ui.input.key.KeyEvent
import androidx.compose.ui.text.input.EditCommand
import co.typie.editor.EditorState
import co.typie.editor.EditorViewportTransform
import co.typie.editor.ffi.CursorMetrics
import co.typie.editor.ffi.Message
import kotlinx.coroutines.CoroutineScope

internal expect class EditorPlatformInputBridge() {
  fun reset()

  fun onPreKeyEvent(
    event: KeyEvent,
    inputCoroutineScope: CoroutineScope,
    onAccepted: () -> Unit,
  ): Boolean

  suspend fun dispatchAppOwnedKeyMessages(
    messages: List<Message>,
    preState: EditorState,
    dispatch: suspend () -> EditorState?,
  )

  fun shouldConsumeKeyEvent(event: KeyEvent): Boolean

  fun interceptEditCommands(commands: List<EditCommand>, state: EditorState): List<Message>?

  fun onImeMessagesCommitted(messages: List<Message>, preState: EditorState, postState: EditorState)

  fun installSessionEffects(
    cursor: () -> CursorMetrics?,
    viewportTransform: () -> EditorViewportTransform,
    dispatch: (List<Message>) -> Unit,
  ): () -> Unit
}
