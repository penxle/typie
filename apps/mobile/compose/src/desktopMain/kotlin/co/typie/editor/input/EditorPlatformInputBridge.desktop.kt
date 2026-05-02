package co.typie.editor.input

import androidx.compose.ui.input.key.KeyEvent
import co.typie.editor.EditorViewportTransform
import co.typie.editor.ffi.CursorMetrics
import co.typie.editor.ffi.ImeRange
import co.typie.editor.ffi.Message
import kotlinx.coroutines.CoroutineScope

internal actual class EditorPlatformInputBridge actual constructor() {
  actual fun reset() = Unit

  actual fun onPreKeyEvent(
    event: KeyEvent,
    selection: ImeRange?,
    inputCoroutineScope: CoroutineScope,
    dispatch: () -> Unit,
  ): Boolean = false

  actual fun shouldConsumeKeyEvent(event: KeyEvent): Boolean = false

  actual fun interceptImeMessages(messages: List<Message>): List<Message> = messages

  actual fun installSessionEffects(
    cursor: () -> CursorMetrics?,
    viewportTransform: () -> EditorViewportTransform,
    dispatch: (List<Message>) -> Unit,
  ): () -> Unit = {}
}
