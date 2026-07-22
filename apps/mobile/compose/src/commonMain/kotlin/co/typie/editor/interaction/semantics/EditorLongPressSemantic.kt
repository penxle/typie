package co.typie.editor.interaction.semantics

import co.typie.editor.Editor
import co.typie.editor.PagePoint
import co.typie.editor.ext.isCollapsed
import co.typie.platform.Platform

internal enum class EditorLongPressSemanticIntent {
  CursorMove,
  WordSelection,
}

internal class EditorLongPressSemantic {
  fun resolveIntent(
    editor: Editor,
    point: PagePoint,
    platform: Platform,
  ): EditorLongPressSemanticIntent {
    if (platform != Platform.Android) {
      return EditorLongPressSemanticIntent.CursorMove
    }
    if (
      editor.selection.isCollapsed() &&
        editor.cursorHitTest(page = point.page, x = point.x, y = point.y)
    ) {
      return EditorLongPressSemanticIntent.CursorMove
    }
    return EditorLongPressSemanticIntent.WordSelection
  }
}
