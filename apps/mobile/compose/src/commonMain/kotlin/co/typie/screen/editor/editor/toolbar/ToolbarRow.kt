package co.typie.screen.editor.editor.toolbar

import androidx.compose.foundation.ScrollState
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.RowScope
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.padding
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import co.typie.ext.horizontalScroll

@Composable
internal fun EditorToolbarRow(
  scope: EditorToolbarPageScope,
  modifier: Modifier = Modifier,
  scrollState: ScrollState? = null,
  content: @Composable RowScope.() -> Unit,
) {
  Box(modifier = modifier.fillMaxSize()) {
    Row(
      modifier =
        Modifier.fillMaxSize()
          .then(scrollState?.let { Modifier.horizontalScroll(it, enabled = false) } ?: Modifier)
          .padding(
            start = ToolbarPageStartPadding,
            top = ToolbarPageVerticalPadding,
            end =
              if (scope.hasNextPage) {
                ToolbarPageIndicatorSlotWidth
              } else {
                ToolbarLastPageReservedEndPadding
              },
            bottom = ToolbarPageVerticalPadding,
          ),
      verticalAlignment = Alignment.CenterVertically,
      horizontalArrangement = Arrangement.spacedBy(ToolbarItemGap),
      content = content,
    )
    if (scope.hasNextPage) {
      EditorToolbarPageIndicator(modifier = Modifier.align(Alignment.CenterEnd))
    }
  }
}
