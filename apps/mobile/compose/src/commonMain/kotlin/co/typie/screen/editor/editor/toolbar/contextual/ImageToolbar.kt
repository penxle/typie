package co.typie.screen.editor.editor.toolbar.contextual

import androidx.compose.runtime.Composable
import androidx.compose.ui.Modifier
import co.typie.editor.ffi.PlainNode
import co.typie.icons.Lucide
import co.typie.screen.editor.editor.toolbar.EditorToolbarButton
import co.typie.screen.editor.editor.toolbar.EditorToolbarPage
import co.typie.screen.editor.editor.toolbar.EditorToolbarPageKey
import co.typie.screen.editor.editor.toolbar.EditorToolbarPageScope
import co.typie.screen.editor.editor.toolbar.EditorToolbarRow

internal fun editorImageToolbarPage(image: PlainNode.Image?): EditorToolbarPage =
  EditorToolbarPage(
    key = EditorToolbarPageKey.Image,
    icon = Lucide.Image,
    contentDescription = "이미지 툴바",
    content = { scope -> EditorImageToolbar(scope = scope, image = image) },
  )

@Composable
private fun EditorImageToolbar(
  scope: EditorToolbarPageScope,
  image: PlainNode.Image?,
  modifier: Modifier = Modifier,
) {
  val hasImage = image?.id != null

  EditorToolbarRow(scope = scope, modifier = modifier) {
    if (!hasImage) {
      EditorToolbarButton(icon = Lucide.Image, contentDescription = "이미지 선택", onClick = {})
    }
    EditorToolbarButton(icon = Lucide.Trash2, contentDescription = "이미지 삭제", onClick = {})
  }
}
