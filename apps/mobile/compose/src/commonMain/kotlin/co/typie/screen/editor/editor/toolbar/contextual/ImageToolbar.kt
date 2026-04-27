package co.typie.screen.editor.editor.toolbar.contextual

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.padding
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.unit.dp
import co.typie.icons.Lucide
import co.typie.screen.editor.editor.toolbar.EditorToolbarButton
import co.typie.screen.editor.editor.toolbar.EditorToolbarPage
import co.typie.screen.editor.editor.toolbar.EditorToolbarPageIndicator
import co.typie.screen.editor.editor.toolbar.EditorToolbarPageKey
import co.typie.screen.editor.editor.toolbar.EditorToolbarPageScope
import co.typie.screen.editor.editor.toolbar.ToolbarItemGap
import co.typie.screen.editor.editor.toolbar.ToolbarLastPageReservedEndPadding
import co.typie.screen.editor.editor.toolbar.ToolbarPageStartPadding
import co.typie.screen.editor.editor.toolbar.ToolbarPageVerticalPadding

internal fun editorImageToolbarPage(): EditorToolbarPage =
  EditorToolbarPage(
    key = EditorToolbarPageKey.Image,
    icon = Lucide.Image,
    contentDescription = "이미지 툴바",
    content = { scope -> EditorImageToolbar(scope = scope) },
  )

@Composable
private fun EditorImageToolbar(scope: EditorToolbarPageScope, modifier: Modifier = Modifier) {
  Row(
    modifier =
      modifier
        .fillMaxSize()
        .padding(
          start = ToolbarPageStartPadding,
          top = ToolbarPageVerticalPadding,
          end = if (scope.hasNextPage) 0.dp else ToolbarLastPageReservedEndPadding,
          bottom = ToolbarPageVerticalPadding,
        ),
    verticalAlignment = Alignment.CenterVertically,
    horizontalArrangement = Arrangement.spacedBy(ToolbarItemGap),
  ) {
    EditorToolbarButton(icon = Lucide.Image, contentDescription = "이미지 선택", onClick = {})
    EditorToolbarButton(icon = Lucide.Trash2, contentDescription = "이미지 삭제", onClick = {})
    if (scope.hasNextPage) {
      EditorToolbarPageIndicator()
    }
  }
}
