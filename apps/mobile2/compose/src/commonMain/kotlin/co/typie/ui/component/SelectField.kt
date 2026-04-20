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
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.graphicsLayer
import androidx.compose.ui.unit.dp
import co.typie.form.FieldState
import co.typie.icons.Lucide
import co.typie.ui.component.popover.PopoverMenu
import co.typie.ui.component.popover.PopoverPlacement
import co.typie.ui.icon.Icon
import co.typie.ui.icon.IconData
import co.typie.ui.theme.AppShapes
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
  val selectedValues = values ?: listOf(field.value)
  val resolvedDisplayItem =
    resolveSelectFieldDisplayItem(
      currentValue = field.value,
      selectedValues = selectedValues,
      items = items,
    )

  val anchor =
    @Composable {
      Box(modifier = modifier) { SelectFieldAnchor(item = resolvedDisplayItem, enabled = enabled) }
    }

  PopoverMenu(
    enabled = enabled,
    placement = placement,
    maxWidth = 320.dp,
    screenPadding = PaddingValues(20.dp),
    collapsedCornerRadius = 8.dp,
    anchor = anchor,
  ) {
    items.forEach { selectItem ->
      item(
        content = {
          SelectFieldPopoverItem(
            item = selectItem,
            selected = selectedValues.contains(selectItem.value),
          )
        }
      ) {
        field.setValue(selectItem.value)
        onSelected(selectItem.value)
      }
    }
  }
}

private fun <T> resolveSelectFieldDisplayItem(
  currentValue: T,
  selectedValues: List<T>,
  items: List<SelectFieldItem<T>>,
): SelectFieldDisplayItem {
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
        .graphicsLayer { alpha = if (enabled) 1f else 0.5f }
        .background(AppTheme.colors.surfaceDefault, AppShapes.rounded(AppShapes.md))
        .border(1.dp, AppTheme.colors.borderEmphasis, AppShapes.rounded(AppShapes.md))
        .padding(horizontal = 12.dp, vertical = 8.dp),
    verticalAlignment = Alignment.CenterVertically,
    horizontalArrangement = Arrangement.spacedBy(8.dp),
  ) {
    if (item.icon != null) {
      Icon(icon = item.icon, modifier = Modifier.size(18.dp), tint = AppTheme.colors.textMuted)
    }

    Text(text = item.label, style = AppTheme.typography.body, color = AppTheme.colors.textMuted)

    Icon(
      icon = Lucide.ChevronDown,
      modifier = Modifier.size(16.dp),
      tint = AppTheme.colors.textMuted,
    )
  }
}

@Composable
private fun <T> SelectFieldPopoverItem(item: SelectFieldItem<T>, selected: Boolean) {
  val labelColor = if (selected) AppTheme.colors.textDefault else AppTheme.colors.textMuted
  val descriptionColor = if (selected) AppTheme.colors.textMuted else AppTheme.colors.textMuted

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
          tint = AppTheme.colors.textDefault,
        )
      } else {
        Spacer(Modifier.width(16.dp))
      }
    }
  }
}
