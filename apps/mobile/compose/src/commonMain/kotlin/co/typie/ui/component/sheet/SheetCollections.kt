package co.typie.ui.component.sheet

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.ColumnScope
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.RowScope
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.heightIn
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.input.key.Key.Companion.T
import androidx.compose.ui.unit.dp
import co.typie.ext.InteractionScope
import co.typie.ext.clickable
import co.typie.ext.pressScale
import co.typie.icons.Lucide
import co.typie.ui.component.CardDivider
import co.typie.ui.component.CardRow
import co.typie.ui.component.CardSurface
import co.typie.ui.component.Text
import co.typie.ui.icon.Icon
import co.typie.ui.icon.IconData
import co.typie.ui.theme.AppTheme

@Composable
fun <T> SheetOptionList(
  items: Iterable<T>,
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
fun SheetOptionRow(
  selected: Boolean,
  onClick: suspend () -> Unit,
  modifier: Modifier = Modifier,
  enabled: Boolean = true,
  contentPadding: PaddingValues = PaddingValues(horizontal = 12.dp, vertical = 16.dp),
  trailing: @Composable RowScope.() -> Unit = {
    if (selected) {
      Icon(icon = Lucide.Check, modifier = Modifier.size(16.dp), tint = AppTheme.colors.textDefault)
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

@Composable
fun SheetActionRow(
  icon: IconData,
  label: String,
  onClick: suspend () -> Unit,
  modifier: Modifier = Modifier,
  tint: Color? = null,
  trailingIcon: IconData? = null,
  contentPadding: PaddingValues = PaddingValues(horizontal = 24.dp, vertical = 12.dp),
) {
  InteractionScope {
    Row(
      modifier =
        modifier
          .fillMaxWidth()
          .clickable(onClick = onClick)
          .heightIn(min = 44.dp)
          .padding(contentPadding)
          .pressScale(),
      verticalAlignment = Alignment.CenterVertically,
    ) {
      Icon(icon = icon, modifier = Modifier.size(18.dp), tint = tint ?: AppTheme.colors.textDefault)

      Spacer(Modifier.size(12.dp))

      Text(
        text = label,
        modifier = Modifier.weight(1f),
        style = AppTheme.typography.action,
        color = tint ?: AppTheme.colors.textDefault,
      )

      if (trailingIcon != null) {
        Icon(
          icon = trailingIcon,
          modifier = Modifier.size(14.dp),
          tint = tint ?: AppTheme.colors.textMuted,
        )
      }
    }
  }
}
