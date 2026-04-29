package co.typie.screen.editor.editor.toolbar.contextual

import co.typie.icons.Lucide
import co.typie.screen.editor.editor.toolbar.EditorToolbarButton
import co.typie.screen.editor.editor.toolbar.EditorToolbarPage
import co.typie.screen.editor.editor.toolbar.EditorToolbarPageKey
import co.typie.screen.editor.editor.toolbar.EditorToolbarRow
import co.typie.screen.editor.editor.toolbar.EditorToolbarTableMode

internal fun editorTableToolbarPage(mode: EditorToolbarTableMode?): EditorToolbarPage =
  EditorToolbarPage(
    key = EditorToolbarPageKey.Table,
    icon = Lucide.Table,
    contentDescription = "표 툴바",
    content = { scope ->
      EditorToolbarRow(scope = scope) {
        if (mode == EditorToolbarTableMode.Selected) {
          EditorToolbarButton(icon = Lucide.Trash2, contentDescription = "표 삭제", onClick = {})
          EditorToolbarButton(icon = Lucide.AlignLeft, contentDescription = "표 정렬", onClick = {})
          EditorToolbarButton(
            icon = Lucide.SquareDashed,
            contentDescription = "표 테두리",
            onClick = {},
          )
        } else {
          EditorToolbarButton(icon = Lucide.GripVertical, contentDescription = "표 선택", onClick = {})
        }
      }
    },
  )
