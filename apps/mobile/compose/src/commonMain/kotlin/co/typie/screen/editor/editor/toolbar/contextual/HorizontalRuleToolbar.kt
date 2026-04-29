package co.typie.screen.editor.editor.toolbar.contextual

import co.typie.icons.Lucide
import co.typie.icons.Typie
import co.typie.screen.editor.editor.toolbar.EditorToolbarButton
import co.typie.screen.editor.editor.toolbar.EditorToolbarPage
import co.typie.screen.editor.editor.toolbar.EditorToolbarPageKey
import co.typie.screen.editor.editor.toolbar.EditorToolbarRow

internal fun editorHorizontalRuleToolbarPage(): EditorToolbarPage =
  EditorToolbarPage(
    key = EditorToolbarPageKey.HorizontalRule,
    icon = Lucide.Scissors,
    contentDescription = "구분선 툴바",
    content = { scope ->
      EditorToolbarRow(scope = scope) {
        EditorToolbarButton(
          icon = Typie.HorizontalRule,
          contentDescription = "구분선 설정",
          onClick = {},
        )
        EditorToolbarButton(icon = Lucide.Trash2, contentDescription = "구분선 삭제", onClick = {})
      }
    },
  )
