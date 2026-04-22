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

  // TODO(editor-parity): selection rect, composition rect, 인라인 맞춤법 하이라이트,
  // 인라인 리마크 하이라이트 같은 에디터 로컬 오버레이를 채워 넣어야 한다.
}

internal fun shouldShowEditorCursorOverlay(focused: Boolean, hasCursor: Boolean): Boolean =
  focused && hasCursor
