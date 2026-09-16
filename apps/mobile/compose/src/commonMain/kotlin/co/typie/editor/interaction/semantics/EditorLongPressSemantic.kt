package co.typie.editor.interaction.semantics

import co.typie.editor.Editor
import co.typie.editor.PagePoint
import co.typie.editor.ext.isCollapsed
import co.typie.platform.Platform

internal enum class EditorLongPressSemanticIntent {
  CursorMove,
  WordSelection,
  ContextMenu,
}

internal class EditorLongPressSemantic {
  fun resolveIntent(
    editor: Editor,
    point: PagePoint,
    platform: Platform,
    editing: Boolean,
  ): EditorLongPressSemanticIntent {
    if (
      platform == Platform.Android &&
        !editor.publishedState.selection.isCollapsed() &&
        editor.selectionHitTest(page = point.page, x = point.x, y = point.y)
    ) {
      return EditorLongPressSemanticIntent.ContextMenu
    }
    if (!editing) {
      return EditorLongPressSemanticIntent.WordSelection
    }
    if (platform != Platform.Android) {
      return EditorLongPressSemanticIntent.CursorMove
    }
    return if (editor.textHitTest(page = point.page, x = point.x, y = point.y)) {
      EditorLongPressSemanticIntent.WordSelection
    } else {
      EditorLongPressSemanticIntent.CursorMove
    }
  }
}
