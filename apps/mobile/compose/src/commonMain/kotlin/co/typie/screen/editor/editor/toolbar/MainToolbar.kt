package co.typie.screen.editor.editor.toolbar

import androidx.compose.foundation.layout.Spacer
import androidx.compose.runtime.Composable
import androidx.compose.ui.Modifier
import co.typie.editor.ffi.HistoryOp
import co.typie.editor.ffi.Message
import co.typie.icons.Lucide
import co.typie.ui.utils.ShortcutModifier
import co.typie.ui.utils.shortcutLabel

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
  val insertPanelOpen = scope.activeBottomPanel == EditorToolbarBottomPanel.Insert
  val toolsPanelOpen = scope.activeBottomPanel == EditorToolbarBottomPanel.Tools

  EditorToolbarRow(scope = scope, modifier = modifier) {
    EditorToolbarButton(
      icon = Lucide.Plus,
      contentDescription = "삽입 도구",
      onClick = { scope.toggleBottomPanel(EditorToolbarBottomPanel.Insert) },
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
      shortcut = shortcutLabel("Z", ShortcutModifier.Mod),
      onClick = { scope.sendMessage(Message.History(HistoryOp.Undo)) },
    )
    EditorToolbarButton(
      icon = Lucide.Redo,
      contentDescription = "다시 실행",
      shortcut = shortcutLabel("Z", ShortcutModifier.Mod, ShortcutModifier.Shift),
      onClick = { scope.sendMessage(Message.History(HistoryOp.Redo)) },
    )
    Spacer(Modifier.weight(1f))
    EditorToolbarButton(
      icon = Lucide.Search,
      contentDescription = "찾기 및 바꾸기",
      shortcut = shortcutLabel("F", ShortcutModifier.Mod),
      onClick = { scope.performToolAction(EditorToolbarToolAction.Search) },
    )
    EditorToolbarButton(
      icon = Lucide.Ellipsis,
      contentDescription = "도구",
      onClick = { scope.toggleBottomPanel(EditorToolbarBottomPanel.Tools) },
      selected = toolsPanelOpen,
    )
  }
}
