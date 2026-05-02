package co.typie.editor.input

import androidx.compose.ui.input.key.KeyEvent
import co.typie.editor.EditorViewportTransform
import co.typie.editor.ffi.CursorMetrics
import co.typie.editor.ffi.ImeRange
import co.typie.editor.ffi.Message
import kotlinx.coroutines.CoroutineScope

internal expect class EditorPlatformInputBridge() {
  fun reset()

  fun onPreKeyEvent(
    event: KeyEvent,
    selection: ImeRange?,
    inputCoroutineScope: CoroutineScope,
    dispatch: () -> Unit,
  ): Boolean

  fun shouldConsumeKeyEvent(event: KeyEvent): Boolean

  fun interceptImeMessages(messages: List<Message>): List<Message>

  fun installSessionEffects(
    cursor: () -> CursorMetrics?,
    viewportTransform: () -> EditorViewportTransform,
    dispatch: (List<Message>) -> Unit,
  ): () -> Unit
}
