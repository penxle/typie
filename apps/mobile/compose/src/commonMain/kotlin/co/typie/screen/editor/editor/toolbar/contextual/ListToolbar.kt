package co.typie.screen.editor.editor.toolbar.contextual

import co.typie.icons.Lucide
import co.typie.screen.editor.editor.toolbar.EditorToolbarButton
import co.typie.screen.editor.editor.toolbar.EditorToolbarDivider
import co.typie.screen.editor.editor.toolbar.EditorToolbarListMode
import co.typie.screen.editor.editor.toolbar.EditorToolbarPage
import co.typie.screen.editor.editor.toolbar.EditorToolbarPageKey
import co.typie.screen.editor.editor.toolbar.EditorToolbarRow

internal fun editorListToolbarPage(mode: EditorToolbarListMode?): EditorToolbarPage =
  EditorToolbarPage(
    key = EditorToolbarPageKey.List,
    icon = Lucide.List,
    contentDescription = "목록 툴바",
    content = { scope ->
      EditorToolbarRow(scope = scope) {
        EditorToolbarButton(
          icon = Lucide.Dot,
          contentDescription = "글머리 목록",
          selected = mode == EditorToolbarListMode.Bullet,
          onClick = {},
        )
        EditorToolbarButton(
          icon = Lucide.Hash,
          contentDescription = "번호 목록",
          selected = mode == EditorToolbarListMode.Ordered,
          onClick = {},
        )
        EditorToolbarDivider()
        EditorToolbarButton(icon = Lucide.IndentIncrease, contentDescription = "들여쓰기", onClick = {})
        EditorToolbarButton(icon = Lucide.IndentDecrease, contentDescription = "내어쓰기", onClick = {})
      }
    },
  )
