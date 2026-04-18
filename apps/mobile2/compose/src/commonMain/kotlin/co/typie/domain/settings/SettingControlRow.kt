package co.typie.domain.settings

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.RowScope
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.unit.dp
import co.typie.ext.InteractionScope
import co.typie.ext.clickable
import co.typie.ext.pressScale
import co.typie.ui.component.Text
import co.typie.ui.theme.AppTheme

@Composable
fun SettingControlRow(
  label: String,
  description: String,
  modifier: Modifier = Modifier,
  enabled: Boolean = true,
  onClick: (suspend () -> Unit)? = null,
  trailing: @Composable RowScope.() -> Unit = {},
) {
  val interactiveModifier =
    if (enabled && onClick != null) {
      Modifier.clickable(onClick).pressScale()
    } else {
      Modifier
    }

  InteractionScope {
    Row(
      modifier =
        modifier
          .fillMaxWidth()
          .then(interactiveModifier)
          .padding(horizontal = 20.dp, vertical = 18.dp),
      horizontalArrangement = Arrangement.spacedBy(16.dp),
      verticalAlignment = Alignment.CenterVertically,
    ) {
      Column(modifier = Modifier.weight(1f), verticalArrangement = Arrangement.spacedBy(4.dp)) {
        Text(text = label, style = AppTheme.typography.label)
        Text(
          text = description,
          style = AppTheme.typography.caption,
          color = AppTheme.colors.textMuted,
        )
      }

      Row(
        horizontalArrangement = Arrangement.spacedBy(8.dp),
        verticalAlignment = Alignment.CenterVertically,
        content = trailing,
      )
    }
  }
}
