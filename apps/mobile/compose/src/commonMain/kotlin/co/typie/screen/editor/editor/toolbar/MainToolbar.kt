package co.typie.screen.editor.editor.toolbar

import androidx.compose.foundation.layout.Spacer
import androidx.compose.runtime.Composable
import androidx.compose.ui.Modifier
import co.typie.editor.ffi.HistoryOp
import co.typie.editor.ffi.Message
import co.typie.icons.Lucide

internal fun editorMainToolbarPage(hasTextPage: Boolean): EditorToolbarPage =
  EditorToolbarPage(
    key = EditorToolbarPageKey.Main,
    icon = Lucide.CircleSmall,
    contentDescription = "메인 툴바",
    content = { scope -> EditorMainToolbar(scope = scope, hasTextPage = hasTextPage) },
  )

@Composable
private fun EditorMainToolbar(
  scope: EditorToolbarPageScope,
  hasTextPage: Boolean,
  modifier: Modifier = Modifier,
) {
  val insertPanelOpen = scope.activeBottomPanel == EditorToolbarBottomPanelKey.Insert
  val toolsPanelOpen = scope.activeBottomPanel == EditorToolbarBottomPanelKey.Tools

  EditorToolbarRow(scope = scope, modifier = modifier) {
    EditorToolbarButton(
      icon = Lucide.Plus,
      contentDescription = "새 노드 삽입",
      onClick = { scope.toggleBottomPanel(EditorToolbarBottomPanelKey.Insert) },
      selected = insertPanelOpen,
    )
    if (hasTextPage) {
      EditorToolbarButton(
        icon = Lucide.Type,
        contentDescription = "텍스트",
        onClick = { scope.navigateToPage(EditorToolbarPageKey.Text) },
      )
    }
    EditorToolbarButton(
      icon = Lucide.Undo,
      contentDescription = "실행 취소",
      onClick = { scope.sendMessage(Message.History(HistoryOp.Undo)) },
    )
    EditorToolbarButton(
      icon = Lucide.Redo,
      contentDescription = "다시 실행",
      onClick = { scope.sendMessage(Message.History(HistoryOp.Redo)) },
    )
    Spacer(Modifier.weight(1f))
    EditorToolbarButton(
      icon = Lucide.Search,
      contentDescription = "찾기",
      onClick = { scope.performToolAction(EditorToolbarToolAction.Search) },
    )
    EditorToolbarButton(
      icon = Lucide.Ellipsis,
      contentDescription = "도구",
      onClick = { scope.toggleBottomPanel(EditorToolbarBottomPanelKey.Tools) },
      selected = toolsPanelOpen,
    )
  }
}
