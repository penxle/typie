package co.typie.screen.editor.editor.toolbar.bottom

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.padding
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import co.typie.icons.Lucide
import co.typie.screen.editor.editor.toolbar.EditorToolbarButton
import co.typie.screen.editor.editor.toolbar.ToolbarItemGap
import co.typie.screen.editor.editor.toolbar.ToolbarPageStartPadding

@Composable
internal fun BottomToolbarTools(modifier: Modifier = Modifier) {
  Row(
    modifier = modifier.fillMaxSize().padding(ToolbarPageStartPadding),
    horizontalArrangement = Arrangement.spacedBy(ToolbarItemGap, Alignment.CenterHorizontally),
    verticalAlignment = Alignment.CenterVertically,
  ) {
    EditorToolbarButton(icon = Lucide.StickyNote, contentDescription = "노트", onClick = {})
    EditorToolbarButton(icon = Lucide.MessageSquareText, contentDescription = "코멘트", onClick = {})
    EditorToolbarButton(icon = Lucide.SpellCheck, contentDescription = "맞춤법 검사", onClick = {})
    EditorToolbarButton(icon = Lucide.Lightbulb, contentDescription = "AI 피드백", onClick = {})
    EditorToolbarButton(icon = Lucide.History, contentDescription = "타임라인", onClick = {})
  }
}
