package co.typie.screen.editor.editor.toolbar.contextual

import co.typie.icons.Lucide
import co.typie.screen.editor.editor.toolbar.EditorToolbarButton
import co.typie.screen.editor.editor.toolbar.EditorToolbarPage
import co.typie.screen.editor.editor.toolbar.EditorToolbarPageKey
import co.typie.screen.editor.editor.toolbar.EditorToolbarRow

internal fun editorBlockquoteToolbarPage(): EditorToolbarPage =
  EditorToolbarPage(
    key = EditorToolbarPageKey.Blockquote,
    icon = Lucide.Quote,
    contentDescription = "인용구 툴바",
    content = { scope ->
      EditorToolbarRow(scope = scope) {
        EditorToolbarButton(icon = Lucide.Quote, contentDescription = "인용구 설정", onClick = {})
        EditorToolbarButton(icon = Lucide.TextSelect, contentDescription = "일반 텍스트로", onClick = {})
      }
    },
  )
