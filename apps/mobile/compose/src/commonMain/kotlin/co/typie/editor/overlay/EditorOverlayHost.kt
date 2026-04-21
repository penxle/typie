package co.typie.editor.overlay

import androidx.compose.runtime.Composable
import androidx.compose.ui.geometry.Size as ComposeSize
import co.typie.editor.runtime.LocalEditorRuntime
import co.typie.editor.runtime.LocalEditorUiState

@Composable
fun EditorOverlayHost() {
  val runtime = LocalEditorRuntime.current
  val uiState = LocalEditorUiState.current
  val editor = runtime.editor ?: return
  val cursor = editor.cursor
  if (shouldShowEditorCursorOverlay(focused = uiState.focused, hasCursor = cursor != null)) {
    val cursorOffsetInViewport =
      uiState.localToGlobal(page = cursor!!.pageIdx, x = cursor.rect.x, y = cursor.rect.y)

    if (cursorOffsetInViewport != null) {
      EditorCursorOverlay(
        offset = cursorOffsetInViewport,
        size = ComposeSize(cursor.rect.width, cursor.rect.height),
      )
    }
  }

  // TODO(editor-parity): Populate editor-local overlays such as selection rects,
  // composition rects, inline spellcheck highlights, and inline remark highlights.
}

internal fun shouldShowEditorCursorOverlay(focused: Boolean, hasCursor: Boolean): Boolean =
  focused && hasCursor
