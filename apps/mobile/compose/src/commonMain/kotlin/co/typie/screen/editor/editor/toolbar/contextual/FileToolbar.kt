package co.typie.screen.editor.editor.toolbar.contextual

import co.typie.icons.Lucide
import co.typie.screen.editor.editor.toolbar.EditorToolbarButton
import co.typie.screen.editor.editor.toolbar.EditorToolbarPage
import co.typie.screen.editor.editor.toolbar.EditorToolbarPageKey
import co.typie.screen.editor.editor.toolbar.EditorToolbarRow

internal fun editorFileToolbarPage(): EditorToolbarPage =
  EditorToolbarPage(
    key = EditorToolbarPageKey.File,
    icon = Lucide.Paperclip,
    contentDescription = "파일 툴바",
    content = { scope ->
      EditorToolbarRow(scope = scope) {
        EditorToolbarButton(icon = Lucide.Paperclip, contentDescription = "파일 첨부", onClick = {})
        EditorToolbarButton(icon = Lucide.Download, contentDescription = "파일 다운로드", onClick = {})
        EditorToolbarButton(icon = Lucide.Trash2, contentDescription = "파일 삭제", onClick = {})
      }
    },
  )
