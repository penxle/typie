package co.typie.ui.component.editorsettings

import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.lazy.LazyRow
import androidx.compose.foundation.lazy.itemsIndexed
import androidx.compose.foundation.lazy.rememberLazyListState
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.unit.dp
import co.typie.editor.EditorOption
import co.typie.ext.clickable
import co.typie.ui.component.Text
import co.typie.ui.theme.AppShapes
import co.typie.ui.theme.AppTheme

@Composable
internal fun <T> EditorSettingsChipRow(
  options: List<EditorOption<T>>,
  selected: T,
  onSelect: suspend (T) -> Unit,
  modifier: Modifier = Modifier,
  trailing: (@Composable () -> Unit)? = null,
) {
  val selectedIndex = remember(options, selected) { options.indexOfFirst { it.value == selected } }
  val listState = rememberLazyListState(initialFirstVisibleItemIndex = maxOf(0, selectedIndex))
  var initialScrollDone by remember { mutableStateOf(false) }

  LaunchedEffect(selectedIndex) {
    if (selectedIndex < 0) return@LaunchedEffect

    if (!initialScrollDone) {
      initialScrollDone = true
      listState.scrollToItem(selectedIndex)
      val item =
        listState.layoutInfo.visibleItemsInfo.firstOrNull { it.index == selectedIndex }
          ?: return@LaunchedEffect
      val layoutInfo = listState.layoutInfo
      val contentStart = layoutInfo.viewportStartOffset + layoutInfo.beforeContentPadding
      val contentEnd = layoutInfo.viewportEndOffset - layoutInfo.afterContentPadding
      val contentCenter = (contentStart + contentEnd) / 2
      val scrollBy = item.offset + item.size / 2 - contentCenter
      if (scrollBy != 0) {
        listState.scrollToItem(
          listState.firstVisibleItemIndex,
          listState.firstVisibleItemScrollOffset + scrollBy,
        )
      }
      return@LaunchedEffect
    }

    val layoutInfo = listState.layoutInfo
    val contentEnd = layoutInfo.viewportEndOffset - layoutInfo.afterContentPadding
    val item = layoutInfo.visibleItemsInfo.firstOrNull { it.index == selectedIndex }
    if (item != null && item.offset >= 0 && item.offset + item.size <= contentEnd)
      return@LaunchedEffect
    if (item == null || item.offset < 0) {
      listState.animateScrollToItem(selectedIndex)
    } else {
      val scrollBy = item.offset + item.size - contentEnd
      listState.animateScrollToItem(
        listState.firstVisibleItemIndex,
        listState.firstVisibleItemScrollOffset + scrollBy,
      )
    }
  }

  LazyRow(
    state = listState,
    modifier = modifier,
    contentPadding = PaddingValues(horizontal = 16.dp),
    horizontalArrangement = Arrangement.spacedBy(8.dp),
  ) {
    itemsIndexed(options) { _, option ->
      val isSelected = option.value == selected

      val backgroundColor =
        if (isSelected) AppTheme.colors.textDefault else AppTheme.colors.surfaceInset
      val textColor =
        if (isSelected) AppTheme.colors.surfaceDefault else AppTheme.colors.textDefault

      Box(
        modifier =
          Modifier.clickable { onSelect(option.value) }
            .background(backgroundColor, AppShapes.circle)
            .padding(horizontal = 16.dp, vertical = 8.dp)
      ) {
        Text(text = option.label, style = AppTheme.typography.action, color = textColor)
      }
    }

    if (trailing != null) {
      item { trailing() }
    }
  }
}
