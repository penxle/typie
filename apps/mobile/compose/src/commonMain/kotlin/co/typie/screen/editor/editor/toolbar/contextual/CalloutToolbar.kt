package co.typie.screen.editor.editor.toolbar.contextual

import co.typie.icons.Lucide
import co.typie.screen.editor.editor.toolbar.EditorToolbarButton
import co.typie.screen.editor.editor.toolbar.EditorToolbarPage
import co.typie.screen.editor.editor.toolbar.EditorToolbarPageKey
import co.typie.screen.editor.editor.toolbar.EditorToolbarRow

internal fun editorCalloutToolbarPage(): EditorToolbarPage =
  EditorToolbarPage(
    key = EditorToolbarPageKey.Callout,
    icon = Lucide.GalleryVerticalEnd,
    contentDescription = "강조 툴바",
    content = { scope ->
      EditorToolbarRow(scope = scope) {
        EditorToolbarButton(
          icon = Lucide.GalleryVerticalEnd,
          contentDescription = "강조 종류",
          onClick = {},
        )
        EditorToolbarButton(icon = Lucide.TextSelect, contentDescription = "일반 텍스트로", onClick = {})
      }
    },
  )
