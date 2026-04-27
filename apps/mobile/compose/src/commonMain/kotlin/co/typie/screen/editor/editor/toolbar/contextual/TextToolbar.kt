package co.typie.screen.editor.editor.toolbar.contextual

import androidx.compose.foundation.ScrollState
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.rememberScrollState
import androidx.compose.runtime.Composable
import androidx.compose.runtime.remember
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.unit.dp
import co.typie.ext.horizontalScroll
import co.typie.icons.Lucide
import co.typie.icons.Typie
import co.typie.screen.editor.editor.toolbar.EditorToolbarButton
import co.typie.screen.editor.editor.toolbar.EditorToolbarDivider
import co.typie.screen.editor.editor.toolbar.EditorToolbarLabelButton
import co.typie.screen.editor.editor.toolbar.EditorToolbarPage
import co.typie.screen.editor.editor.toolbar.EditorToolbarPageIndicator
import co.typie.screen.editor.editor.toolbar.EditorToolbarPageKey
import co.typie.screen.editor.editor.toolbar.EditorToolbarPageScope
import co.typie.screen.editor.editor.toolbar.ToolbarItemGap
import co.typie.screen.editor.editor.toolbar.ToolbarLastPageReservedEndPadding
import co.typie.screen.editor.editor.toolbar.ToolbarPageStartPadding
import co.typie.screen.editor.editor.toolbar.ToolbarPageVerticalPadding

@Composable
internal fun rememberTextToolbarPage(): EditorToolbarPage {
  val scrollState = rememberScrollState()
  return remember(scrollState) {
    EditorToolbarPage(
      key = EditorToolbarPageKey.Text,
      icon = Lucide.Type,
      contentDescription = "텍스트 툴바",
      scrollState = scrollState,
      content = { scope -> EditorTextToolbar(scope = scope, scrollState = scrollState) },
    )
  }
}

@Composable
private fun EditorTextToolbar(
  scope: EditorToolbarPageScope,
  scrollState: ScrollState,
  modifier: Modifier = Modifier,
) {
  Row(
    modifier =
      modifier
        .fillMaxSize()
        .horizontalScroll(scrollState, enabled = false)
        .padding(
          start = ToolbarPageStartPadding,
          top = ToolbarPageVerticalPadding,
          end = if (scope.hasNextPage) 0.dp else ToolbarLastPageReservedEndPadding,
          bottom = ToolbarPageVerticalPadding,
        ),
    verticalAlignment = Alignment.CenterVertically,
    horizontalArrangement = Arrangement.spacedBy(ToolbarItemGap),
  ) {
    EditorToolbarButton(icon = Lucide.Palette, contentDescription = "글자색", onClick = {})
    EditorToolbarButton(icon = Lucide.PaintBucket, contentDescription = "배경색", onClick = {})
    EditorToolbarLabelButton(text = "Pretendard", contentDescription = "폰트 패밀리", onClick = {})
    EditorToolbarLabelButton(text = "보통", contentDescription = "폰트 굵기", onClick = {})
    EditorToolbarLabelButton(text = "16", contentDescription = "폰트 크기", onClick = {})
    EditorToolbarDivider()
    EditorToolbarButton(icon = Lucide.Bold, contentDescription = "굵게", onClick = {})
    EditorToolbarButton(icon = Lucide.Italic, contentDescription = "기울임", onClick = {})
    EditorToolbarButton(icon = Lucide.Underline, contentDescription = "밑줄", onClick = {})
    EditorToolbarButton(icon = Lucide.Strikethrough, contentDescription = "취소선", onClick = {})
    EditorToolbarDivider()
    EditorToolbarButton(icon = Lucide.Link, contentDescription = "링크", onClick = {})
    EditorToolbarButton(icon = Typie.Ruby, contentDescription = "루비", onClick = {})
    EditorToolbarDivider()
    EditorToolbarButton(icon = Lucide.AlignLeft, contentDescription = "문단 정렬", onClick = {})
    EditorToolbarButton(icon = Typie.LineHeight, contentDescription = "줄 높이", onClick = {})
    EditorToolbarButton(icon = Typie.LetterSpacing, contentDescription = "자간", onClick = {})
    EditorToolbarDivider()
    EditorToolbarButton(icon = Lucide.RemoveFormatting, contentDescription = "서식 지우기", onClick = {})
    if (scope.hasNextPage) {
      EditorToolbarPageIndicator()
    }
  }
}
