package co.typie.ui.component.bottomsheet

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.ColumnScope
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.RowScope
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.size
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.unit.dp
import co.typie.icons.Lucide
import co.typie.ui.component.CardDivider
import co.typie.ui.component.CardRow
import co.typie.ui.component.CardSurface
import co.typie.ui.icon.Icon
import co.typie.ui.theme.AppTheme

@Composable
fun <T> BottomSheetOptionList(
  items: List<T>,
  modifier: Modifier = Modifier,
  itemContent: @Composable (T) -> Unit,
) {
  CardSurface(modifier = modifier.fillMaxWidth()) {
    Column(modifier = Modifier.fillMaxWidth()) {
      items.forEachIndexed { index, item ->
        if (index > 0) {
          CardDivider()
        }

        itemContent(item)
      }
    }
  }
}

@Composable
fun BottomSheetOptionRow(
  selected: Boolean,
  onClick: suspend () -> Unit,
  modifier: Modifier = Modifier,
  enabled: Boolean = true,
  contentPadding: PaddingValues = PaddingValues(horizontal = 12.dp, vertical = 16.dp),
  trailing: @Composable RowScope.() -> Unit = {
    if (selected) {
      Icon(icon = Lucide.Check, modifier = Modifier.size(16.dp), tint = AppTheme.colors.brand)
    } else {
      Spacer(Modifier.size(16.dp))
    }
  },
  label: @Composable ColumnScope.() -> Unit,
) {
  CardRow(
    onClick = { if (enabled) onClick() },
    modifier = modifier,
    contentPadding = contentPadding,
  ) {
    Row(
      modifier = Modifier.fillMaxWidth(),
      verticalAlignment = Alignment.CenterVertically,
      horizontalArrangement = Arrangement.spacedBy(12.dp),
    ) {
      Column(
        modifier = Modifier.weight(1f),
        verticalArrangement = Arrangement.spacedBy(4.dp),
        content = label,
      )

      Row(
        horizontalArrangement = Arrangement.spacedBy(8.dp),
        verticalAlignment = Alignment.CenterVertically,
        content = trailing,
      )
    }
  }
}
