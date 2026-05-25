package co.typie.editor.input

import androidx.compose.ui.input.key.KeyEvent
import androidx.compose.ui.text.input.EditCommand
import co.typie.editor.EditorState
import co.typie.editor.EditorViewportTransform
import co.typie.editor.ffi.CursorMetrics
import co.typie.editor.ffi.Message
import kotlinx.coroutines.CoroutineScope

internal actual class EditorPlatformInputBridge actual constructor() {
  actual fun reset() = Unit

  actual fun onPreKeyEvent(
    event: KeyEvent,
    editorState: () -> EditorState,
    inputCoroutineScope: CoroutineScope,
    bindingMessages: suspend () -> List<Message>,
    commit: suspend (List<Message>) -> EditorState?,
  ): Boolean = false

  actual fun shouldConsumeKeyEvent(event: KeyEvent): Boolean = false

  actual fun interceptEditCommands(
    commands: List<EditCommand>,
    state: EditorState,
  ): List<Message>? = null

  actual fun onImeMessagesCommitted(
    messages: List<Message>,
    preState: EditorState,
    postState: EditorState,
  ) = Unit

  actual fun installSessionEffects(
    cursor: () -> CursorMetrics?,
    viewportTransform: () -> EditorViewportTransform,
    dispatch: (List<Message>) -> Unit,
  ): () -> Unit = {}
}
