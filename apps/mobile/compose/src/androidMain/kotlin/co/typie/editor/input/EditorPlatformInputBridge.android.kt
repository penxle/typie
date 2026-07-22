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
