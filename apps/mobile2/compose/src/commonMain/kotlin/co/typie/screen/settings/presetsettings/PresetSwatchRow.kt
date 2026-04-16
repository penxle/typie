package co.typie.screen.settings.presetsettings

import androidx.compose.foundation.Canvas
import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.lazy.LazyRow
import androidx.compose.foundation.lazy.itemsIndexed
import androidx.compose.foundation.lazy.rememberLazyListState
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.unit.Dp
import androidx.compose.ui.unit.dp
import co.typie.editor.EditorColorOption
import co.typie.editor.ResolvedEditorTheme
import co.typie.ext.clickable
import co.typie.ui.theme.AppTheme

@Composable
internal fun PresetSwatchRow(
  options: List<EditorColorOption>,
  selected: String,
  onSelect: suspend (String) -> Unit,
  theme: ResolvedEditorTheme,
  cornerRadius: Dp,
  modifier: Modifier = Modifier,
) {
  val swatchSize = 28.dp
  val ringWidth = 2.dp
  val ringGap = 2.dp
  val ringInset = ringWidth + ringGap
  val outerShape = RoundedCornerShape(cornerRadius)
  val innerShape = RoundedCornerShape((cornerRadius - ringInset).coerceAtLeast(0.dp))
  val slashColor = AppTheme.colors.textTertiary

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
    horizontalArrangement = Arrangement.spacedBy(4.dp),
  ) {
    itemsIndexed(options) { _, option ->
      val isSelected = option.value == selected
      val color = option.themeKey?.let { theme[it] }

      Box(
        modifier =
          Modifier.clickable { onSelect(option.value) }
            .then(
              if (isSelected) Modifier.border(ringWidth, AppTheme.colors.textPrimary, outerShape)
              else Modifier
            )
            .padding(ringInset)
      ) {
        if (color != null) {
          val needsBorder = color.red > 0.9f && color.green > 0.9f && color.blue > 0.9f
          Box(
            modifier =
              Modifier.size(swatchSize)
                .then(
                  if (needsBorder) Modifier.border(1.dp, AppTheme.colors.borderDefault, innerShape)
                  else Modifier
                )
                .background(color, innerShape)
          )
        } else {
          Box(
            modifier =
              Modifier.size(swatchSize)
                .border(1.dp, AppTheme.colors.borderDefault, innerShape)
                .clip(innerShape)
          ) {
            Canvas(modifier = Modifier.matchParentSize()) {
              drawLine(
                color = slashColor,
                start = Offset(0f, size.height),
                end = Offset(size.width, 0f),
                strokeWidth = 1.5.dp.toPx(),
              )
            }
          }
        }
      }
    }
  }
}
