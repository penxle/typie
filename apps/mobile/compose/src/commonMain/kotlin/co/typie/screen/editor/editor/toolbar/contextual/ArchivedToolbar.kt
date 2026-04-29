package co.typie.screen.editor.editor.toolbar.contextual

import co.typie.icons.Lucide
import co.typie.screen.editor.editor.toolbar.EditorToolbarButton
import co.typie.screen.editor.editor.toolbar.EditorToolbarPage
import co.typie.screen.editor.editor.toolbar.EditorToolbarPageKey
import co.typie.screen.editor.editor.toolbar.EditorToolbarRow

internal fun editorArchivedToolbarPage(): EditorToolbarPage =
  EditorToolbarPage(
    key = EditorToolbarPageKey.Archived,
    icon = Lucide.Eye,
    contentDescription = "보관된 블록 툴바",
    content = { scope ->
      EditorToolbarRow(scope = scope) {
        EditorToolbarButton(icon = Lucide.Eye, contentDescription = "보관된 블록 보기", onClick = {})
      }
    },
  )
