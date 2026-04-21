package co.typie.editor.input

import androidx.compose.foundation.gestures.detectTapGestures
import androidx.compose.ui.Modifier
import androidx.compose.ui.input.pointer.pointerInput
import co.typie.editor.Editor
import co.typie.editor.ffi.Message
import co.typie.editor.ffi.PointerEvent
import co.typie.editor.runtime.EditorUiState

internal fun Modifier.editorGestures(editor: Editor, uiState: EditorUiState): Modifier =
  this.pointerInput(editor, uiState, editor.pageSizes) {
    detectTapGestures { offset ->
      editor.focus()

      val xDp = offset.x / density
      val yDp = offset.y / density
      val point = uiState.globalToLocal(xDp, yDp, editor.pageSizes) ?: return@detectTapGestures

      editor.enqueue(
        Message.Pointer(PointerEvent.Down(page = point.page, x = point.x, y = point.y, count = 1))
      )
    }
  }
