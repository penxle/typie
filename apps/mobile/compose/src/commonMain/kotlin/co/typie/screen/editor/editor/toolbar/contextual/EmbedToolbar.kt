package co.typie.screen.editor.editor.toolbar.contextual

import co.typie.icons.Lucide
import co.typie.screen.editor.editor.toolbar.EditorToolbarButton
import co.typie.screen.editor.editor.toolbar.EditorToolbarPage
import co.typie.screen.editor.editor.toolbar.EditorToolbarPageKey
import co.typie.screen.editor.editor.toolbar.EditorToolbarRow

internal fun editorEmbedToolbarPage(): EditorToolbarPage =
  EditorToolbarPage(
    key = EditorToolbarPageKey.Embed,
    icon = Lucide.FileUp,
    contentDescription = "임베드 툴바",
    content = { scope ->
      EditorToolbarRow(scope = scope) {
        EditorToolbarButton(icon = Lucide.FileUp, contentDescription = "임베드 삽입", onClick = {})
        EditorToolbarButton(
          icon = Lucide.ExternalLink,
          contentDescription = "외부 링크 열기",
          onClick = {},
        )
        EditorToolbarButton(icon = Lucide.Trash2, contentDescription = "임베드 삭제", onClick = {})
      }
    },
  )
