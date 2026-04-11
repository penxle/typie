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
import androidx.compose.ui.text.TextStyle
import androidx.compose.ui.unit.Dp
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
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
fun SheetActionList(modifier: Modifier = Modifier, content: @Composable ColumnScope.() -> Unit) {
  CardSurface(modifier = modifier.fillMaxWidth()) {
    Column(modifier = Modifier.fillMaxWidth(), content = content)
  }
}

@Composable
fun SheetActionRow(
  onClick: suspend () -> Unit,
  modifier: Modifier = Modifier,
  enabled: Boolean = true,
  contentPadding: PaddingValues = PaddingValues(horizontal = 12.dp, vertical = 16.dp),
  content: @Composable RowScope.() -> Unit,
) {
  CardRow(
    onClick = { if (enabled) onClick() },
    modifier = modifier,
    contentPadding = contentPadding,
    content = content,
  )
}

@Composable
fun SheetActionDivider(modifier: Modifier = Modifier) {
  CardDivider(modifier = modifier)
}

@Composable
fun <T> SheetOptionList(
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
fun SheetOptionRow(
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

private val SheetEntityMetadataTextStyle: TextStyle
  @Composable get() = AppTheme.typography.caption.copy(fontSize = 14.sp, lineHeight = 20.sp)

@Composable
fun SheetMenu(
  modifier: Modifier = Modifier,
  header: (@Composable ColumnScope.() -> Unit)? = null,
  showHeaderDivider: Boolean = true,
  content: @Composable ColumnScope.() -> Unit,
) {
  Column(modifier = modifier.fillMaxWidth()) {
    if (header != null) {
      Column(
        modifier = Modifier.fillMaxWidth(),
        verticalArrangement = Arrangement.spacedBy(12.dp),
        content = header,
      )
      Spacer(Modifier.size(if (showHeaderDivider) 16.dp else 8.dp))
      if (showHeaderDivider) {
        SheetMenuDivider()
        Spacer(Modifier.size(8.dp))
      }
    }

    Column(modifier = Modifier.fillMaxWidth(), content = content)
  }
}

@Composable
fun SheetMenuDivider(
  modifier: Modifier = Modifier,
  inset: Dp = 0.dp,
  color: Color = AppTheme.colors.borderDefault,
) {
  CardDivider(modifier = modifier, inset = inset, color = color)
}

@Composable
fun SheetMenuActionRow(
  icon: IconData,
  label: String,
  onClick: () -> Unit,
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
          .clickable { onClick() }
          .heightIn(min = 44.dp)
          .padding(contentPadding)
          .pressScale(),
      verticalAlignment = Alignment.CenterVertically,
    ) {
      Icon(icon = icon, modifier = Modifier.size(18.dp), tint = tint ?: AppTheme.colors.textPrimary)

      Spacer(Modifier.size(12.dp))

      Text(
        text = label,
        modifier = Modifier.weight(1f),
        style = AppTheme.typography.action,
        color = tint ?: AppTheme.colors.textPrimary,
      )

      if (trailingIcon != null) {
        Icon(
          icon = trailingIcon,
          modifier = Modifier.size(14.dp),
          tint = tint ?: AppTheme.colors.textTertiary,
        )
      }
    }
  }
}

@Composable
fun SheetEntityHeader(
  title: String,
  icon: IconData,
  modifier: Modifier = Modifier,
  iconTint: Color = AppTheme.colors.textPrimary,
  supportingContent: (@Composable ColumnScope.() -> Unit)? = null,
) {
  EntityHeader(
    title = title,
    icon = icon,
    modifier = modifier,
    iconTint = iconTint,
    supportingContent = supportingContent,
  )
}

@Composable
fun SheetEntityBreadcrumb(
  segments: List<String>,
  modifier: Modifier = Modifier,
  textStyle: TextStyle = SheetEntityMetadataTextStyle,
  color: Color = AppTheme.colors.textTertiary,
) {
  EntityBreadcrumb(segments = segments, modifier = modifier, textStyle = textStyle, color = color)
}

@Composable
fun SheetEntitySupportingText(
  text: String,
  modifier: Modifier = Modifier,
  color: Color = AppTheme.colors.textMuted,
  textStyle: TextStyle = SheetEntityMetadataTextStyle,
) {
  EntitySupportingText(text = text, modifier = modifier, color = color, textStyle = textStyle)
}
