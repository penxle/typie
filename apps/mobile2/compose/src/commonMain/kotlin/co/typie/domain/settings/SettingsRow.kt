package co.typie.domain.settings

import androidx.compose.foundation.layout.RowScope
import androidx.compose.foundation.layout.size
import androidx.compose.runtime.Composable
import androidx.compose.ui.Modifier
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import co.typie.icons.Lucide
import co.typie.ui.component.CardRow
import co.typie.ui.component.Text
import co.typie.ui.icon.Icon
import co.typie.ui.theme.AppTheme

@Composable
fun SettingsCardRow(
  label: String,
  trailing: @Composable RowScope.() -> Unit = {
    Icon(
      icon = Lucide.ChevronRight,
      modifier = Modifier.size(16.dp),
      tint = AppTheme.colors.textTertiary,
    )
  },
  onClick: suspend () -> Unit,
) {
  CardRow(onClick = onClick) {
    Text(
      text = label,
      style = AppTheme.typography.label,
      modifier = Modifier.weight(1f),
      maxLines = 1,
      overflow = TextOverflow.Ellipsis,
    )

    trailing()
  }
}
