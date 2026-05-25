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
    editorState: () -> EditorState,
    inputCoroutineScope: CoroutineScope,
    bindingMessages: suspend () -> List<Message>,
    commit: suspend (List<Message>) -> EditorState?,
  ): Boolean

  fun shouldConsumeKeyEvent(event: KeyEvent): Boolean

  fun interceptEditCommands(commands: List<EditCommand>, state: EditorState): List<Message>?

  fun onImeMessagesCommitted(messages: List<Message>, preState: EditorState, postState: EditorState)

  fun installSessionEffects(
    cursor: () -> CursorMetrics?,
    viewportTransform: () -> EditorViewportTransform,
    dispatch: (List<Message>) -> Unit,
  ): () -> Unit
}
