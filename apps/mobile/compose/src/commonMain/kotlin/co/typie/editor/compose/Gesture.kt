package co.typie.editor.compose

import androidx.compose.foundation.gestures.detectTapGestures
import androidx.compose.ui.Modifier
import androidx.compose.ui.input.pointer.pointerInput
import co.typie.editor.Editor
import co.typie.editor.ffi.Message
import co.typie.editor.ffi.PointerEvent

internal fun Modifier.editorGestures(editor: Editor): Modifier =
  this.pointerInput(editor, editor.pageSizes) {
    detectTapGestures { offset ->
      editor.focus()

      val xDp = offset.x / density
      val yDp = offset.y / density
      val point = editor.globalToLocal(xDp, yDp) ?: return@detectTapGestures

      editor.enqueue(
        Message.Pointer(PointerEvent.Down(page = point.page, x = point.x, y = point.y, count = 1))
      )
    }
  }
