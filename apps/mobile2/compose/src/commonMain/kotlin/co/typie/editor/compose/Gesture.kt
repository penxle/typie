package co.typie.editor.compose

import androidx.compose.foundation.gestures.detectTapGestures
import androidx.compose.ui.Modifier
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.input.pointer.pointerInput
import androidx.compose.ui.platform.LocalDensity
import androidx.compose.ui.unit.Density
import co.typie.editor.Editor
import co.typie.editor.ffi.Message
import co.typie.editor.ffi.PointerEvent
import co.typie.editor.ffi.Size

internal fun Modifier.editorGestures(
  editor: Editor,
  pageOffsets: Map<Int, Offset>,
  pageSizes: List<Size>,
): Modifier = this.pointerInput(editor, pageOffsets, pageSizes) {
  detectTapGestures { offset ->
    val xDp = offset.x / density
    val yDp = offset.y / density
    val point = globalToLocal(xDp, yDp, pageOffsets, pageSizes) ?: return@detectTapGestures
    editor.enqueue(
      Message.Pointer(
        PointerEvent.Down(
          page = point.page.toLong(),
          x = point.x,
          y = point.y,
          count = 1,
        )
      )
    )
  }
}
