package co.typie.screen.editor.editor.toolbar.bottom

import androidx.compose.foundation.border
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.unit.Dp
import co.typie.icons.Lucide
import co.typie.screen.editor.editor.toolbar.EditorToolbarBottomPanelKey
import co.typie.screen.editor.editor.toolbar.EditorToolbarButton
import co.typie.screen.editor.editor.toolbar.EditorToolbarSurfaceBackground
import co.typie.screen.editor.editor.toolbar.ToolbarBorderWidth
import co.typie.screen.editor.editor.toolbar.ToolbarBottomPanelShape
import co.typie.screen.editor.editor.toolbar.ToolbarItemGap
import co.typie.screen.editor.editor.toolbar.ToolbarPageStartPadding
import co.typie.ui.theme.AppTheme
import co.typie.ui.theme.shadow

@Composable
internal fun EditorToolbarBottomPanel(
  panel: EditorToolbarBottomPanelKey,
  height: Dp,
  modifier: Modifier = Modifier,
) {
  Box(
    modifier =
      modifier
        .fillMaxWidth()
        .height(height)
        .shadow(AppTheme.shadows.sm, ToolbarBottomPanelShape)
        .border(ToolbarBorderWidth, AppTheme.colors.borderEmphasis, ToolbarBottomPanelShape)
  ) {
    EditorToolbarSurfaceBackground(shape = ToolbarBottomPanelShape)

    Row(
      modifier = Modifier.fillMaxSize().padding(ToolbarPageStartPadding),
      horizontalArrangement = Arrangement.spacedBy(ToolbarItemGap, Alignment.CenterHorizontally),
      verticalAlignment = Alignment.CenterVertically,
    ) {
      when (panel) {
        EditorToolbarBottomPanelKey.Insert -> BottomPanelNodes()
        EditorToolbarBottomPanelKey.More -> BottomPanelEntries()
      }
    }
  }
}

@Composable
private fun BottomPanelEntries() {
  EditorToolbarButton(icon = Lucide.StickyNote, contentDescription = "노트", onClick = {})
  EditorToolbarButton(icon = Lucide.MessageSquareText, contentDescription = "코멘트", onClick = {})
  EditorToolbarButton(icon = Lucide.SpellCheck, contentDescription = "맞춤법 검사", onClick = {})
  EditorToolbarButton(icon = Lucide.Lightbulb, contentDescription = "AI 피드백", onClick = {})
  EditorToolbarButton(icon = Lucide.History, contentDescription = "타임라인", onClick = {})
}
