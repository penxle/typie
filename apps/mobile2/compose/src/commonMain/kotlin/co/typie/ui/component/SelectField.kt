package co.typie.ui.component

import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.heightIn
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.runtime.Composable
import androidx.compose.runtime.remember
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.alpha
import androidx.compose.ui.unit.dp
import co.typie.form.FieldState
import co.typie.icons.Lucide
import co.typie.ui.component.popover.Popover
import co.typie.ui.component.popover.PopoverDefaults
import co.typie.ui.component.popover.PopoverList
import co.typie.ui.component.popover.PopoverListItem
import co.typie.ui.component.popover.PopoverPlacement
import co.typie.ui.icon.Icon
import co.typie.ui.icon.IconData
import co.typie.ui.theme.AppTheme

data class SelectFieldItem<T>(
  val value: T,
  val label: String,
  val icon: IconData? = null,
  val description: String? = null,
)

private data class SelectFieldDisplayItem(val label: String, val icon: IconData? = null)

@Composable
fun <T> SelectField(
  field: FieldState<T>,
  items: List<SelectFieldItem<T>>,
  modifier: Modifier = Modifier,
  enabled: Boolean = true,
  values: List<T>? = null,
  placement: PopoverPlacement = PopoverPlacement.BelowEnd,
  onSelected: (T) -> Unit = {},
) {
  val resolvedDisplayItem =
    remember(field.value, values, items) {
      resolveSelectFieldDisplayItem(currentValue = field.value, values = values, items = items)
    }

  val anchor =
    @Composable {
      Box(modifier = modifier) { SelectFieldAnchor(item = resolvedDisplayItem, enabled = enabled) }
    }

  if (!enabled) {
    anchor()
    return
  }

  Popover(
    placement = placement,
    maxWidth = 320.dp,
    screenPadding = PaddingValues(20.dp),
    collapsedCornerRadius = 8.dp,
    anchor = anchor,
    pane = {
      Column(modifier = Modifier.padding(PopoverDefaults.PanePadding)) {
        PopoverList(
          items =
            items.map { item ->
              PopoverListItem(
                content = {
                  SelectFieldPopoverItem(item = item, selected = item.value == field.value)
                },
                onSelected = {
                  field.setValue(item.value)
                  close()
                  onSelected(item.value)
                },
              )
            }
        )
      }
    },
  )
}

private fun <T> resolveSelectFieldDisplayItem(
  currentValue: T,
  values: List<T>?,
  items: List<SelectFieldItem<T>>,
): SelectFieldDisplayItem {
  val selectedValues = values ?: listOf(currentValue)
  val distinctValues = selectedValues.distinct()

  if (distinctValues.size > 1) {
    return SelectFieldDisplayItem(
      icon = Lucide.Minus,
      label =
        distinctValues.joinToString(", ") { value ->
          items.firstOrNull { it.value == value }?.label ?: ""
        },
    )
  }

  val selectedItem = items.firstOrNull { it.value == currentValue }
  return SelectFieldDisplayItem(
    icon = selectedItem?.icon,
    label = selectedItem?.label ?: "(알 수 없음)",
  )
}

@Composable
private fun SelectFieldAnchor(item: SelectFieldDisplayItem, enabled: Boolean) {
  Row(
    modifier =
      Modifier.heightIn(min = 38.dp)
        .alpha(if (enabled) 1f else 0.5f)
        .background(AppTheme.colors.surfaceDefault, RoundedCornerShape(8.dp))
        .border(1.dp, AppTheme.colors.borderStrong, RoundedCornerShape(8.dp))
        .padding(horizontal = 12.dp, vertical = 8.dp),
    verticalAlignment = Alignment.CenterVertically,
    horizontalArrangement = Arrangement.spacedBy(8.dp),
  ) {
    if (item.icon != null) {
      Icon(icon = item.icon, modifier = Modifier.size(18.dp), tint = AppTheme.colors.textSecondary)
    }

    Text(text = item.label, style = AppTheme.typography.body, color = AppTheme.colors.textSecondary)

    Icon(
      icon = Lucide.ChevronDown,
      modifier = Modifier.size(16.dp),
      tint = AppTheme.colors.textTertiary,
    )
  }
}

@Composable
private fun <T> SelectFieldPopoverItem(item: SelectFieldItem<T>, selected: Boolean) {
  val labelColor = if (selected) AppTheme.colors.textPrimary else AppTheme.colors.textSecondary
  val descriptionColor =
    if (selected) AppTheme.colors.textSecondary else AppTheme.colors.textTertiary

  Row(
    modifier = Modifier.fillMaxWidth().padding(horizontal = 14.dp, vertical = 12.dp),
    verticalAlignment = if (item.description == null) Alignment.CenterVertically else Alignment.Top,
    horizontalArrangement = Arrangement.spacedBy(10.dp),
  ) {
    if (item.icon != null) {
      Icon(icon = item.icon, modifier = Modifier.size(18.dp), tint = labelColor)
    }

    Column(modifier = Modifier.weight(1f), verticalArrangement = Arrangement.spacedBy(2.dp)) {
      Text(text = item.label, style = AppTheme.typography.body, color = labelColor)

      if (item.description != null) {
        Text(text = item.description, style = AppTheme.typography.caption, color = descriptionColor)
      }
    }

    Box(modifier = Modifier.size(16.dp), contentAlignment = Alignment.Center) {
      if (selected) {
        Icon(
          icon = Lucide.Check,
          modifier = Modifier.size(16.dp),
          tint = AppTheme.colors.textPrimary,
        )
      } else {
        Spacer(Modifier.width(16.dp))
      }
    }
  }
}
