package co.typie.screen.editor.editor.toolbar

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.width
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.unit.dp
import co.typie.icons.Lucide

internal fun editorMainToolbarPage(): EditorToolbarPage =
  EditorToolbarPage(
    key = EditorToolbarPageKey.Main,
    icon = Lucide.CircleSmall,
    contentDescription = "메인 툴바",
    content = { scope -> EditorMainToolbar(scope) },
  )

@Composable
private fun EditorMainToolbar(scope: EditorToolbarPageScope, modifier: Modifier = Modifier) {
  val insertPanelOpen = scope.activeBottomPanel == EditorToolbarBottomPanelKey.Insert
  val morePanelOpen = scope.activeBottomPanel == EditorToolbarBottomPanelKey.More

  Row(
    modifier =
      modifier
        .fillMaxSize()
        .padding(
          start = ToolbarPageStartPadding,
          top = ToolbarPageVerticalPadding,
          end = 0.dp,
          bottom = ToolbarPageVerticalPadding,
        ),
    verticalAlignment = Alignment.CenterVertically,
    horizontalArrangement = Arrangement.spacedBy(ToolbarItemGap),
  ) {
    EditorToolbarButton(
      icon = Lucide.Plus,
      contentDescription = "새 노드 삽입",
      onClick = { scope.toggleBottomPanel(EditorToolbarBottomPanelKey.Insert) },
      selected = insertPanelOpen,
    )
    EditorToolbarButton(
      icon = Lucide.Type,
      contentDescription = "텍스트",
      onClick = { scope.navigateToPage(EditorToolbarPageKey.Text) },
    )
    EditorToolbarButton(icon = Lucide.Undo, contentDescription = "실행 취소", onClick = {})
    EditorToolbarButton(icon = Lucide.Redo, contentDescription = "다시 실행", onClick = {})
    Spacer(Modifier.weight(1f))
    EditorToolbarButton(icon = Lucide.Search, contentDescription = "찾기", onClick = {})
    EditorToolbarButton(
      icon = Lucide.Ellipsis,
      contentDescription = "패널",
      onClick = { scope.toggleBottomPanel(EditorToolbarBottomPanelKey.More) },
      selected = morePanelOpen,
    )
    if (scope.hasNextPage) {
      EditorToolbarPageIndicator()
    } else {
      Spacer(Modifier.width(ToolbarLastPageReservedEndPadding))
    }
  }
}
