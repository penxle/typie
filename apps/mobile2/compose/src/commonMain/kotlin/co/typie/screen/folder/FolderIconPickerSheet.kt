package co.typie.screen.folder

import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.BoxWithConstraints
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.heightIn
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.verticalScroll
import androidx.compose.runtime.Composable
import androidx.compose.runtime.remember
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.graphics.Brush
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import co.typie.ext.InteractionScope
import co.typie.ext.clickable
import co.typie.ext.pressScale
import co.typie.ext.safeBottomPadding
import co.typie.form.FormState
import co.typie.icons.Lucide
import co.typie.screen.home.EntityIconColorOption
import co.typie.screen.home.EntityIconOption
import co.typie.screen.home.entityIconColors
import co.typie.screen.home.entityIcons
import co.typie.screen.home.resolveEntityIconTint
import co.typie.ui.component.bottomsheet.BottomSheetHeaderTextAction
import co.typie.ui.component.bottomsheet.BottomSheetScaffold
import co.typie.ui.component.bottomsheet.BottomSheetScope
import co.typie.ui.component.bottomsheet.dismiss
import co.typie.ui.icon.Icon
import co.typie.ui.theme.AppTheme
import kotlinx.coroutines.CoroutineScope

private const val DEFAULT_ENTITY_ICON_NAME = "folder"
private const val DEFAULT_ENTITY_ICON_COLOR = "gray"
private val FolderIconPickerCellSpacing = 2.dp
private val FolderIconPickerGridIconSize = 18.dp
private val FolderIconPickerGridMaxHeight = 240.dp
private val FolderIconPickerSelectionDotSize = 4.dp
private val FolderIconPickerSelectionDotBottomInset = 4.dp
private val FolderIconPickerGridBottomInset = 12.dp
private val FolderIconPickerTopFadeHeight = 16.dp

private class FolderIconPickerForm(
  scope: CoroutineScope,
  initialIconName: String,
  initialColor: String,
) : FormState(scope) {
  val iconName = field(initialIconName) {
    focusable = false
  }
  val color = field(initialColor) {
    focusable = false
  }
}

@Composable
fun BottomSheetScope<Unit>.FolderIconPickerSheet(
  model: FolderViewModel,
  entityId: String,
  initialIcon: String?,
  initialColor: String?,
  onUpdated: () -> Unit = {},
) {
  val normalizedInitialIcon = initialIcon?.trim()?.takeIf { it.isNotEmpty() } ?: DEFAULT_ENTITY_ICON_NAME
  val normalizedInitialColor = initialColor?.trim()?.takeIf { it.isNotEmpty() } ?: DEFAULT_ENTITY_ICON_COLOR
  val scope = rememberCoroutineScope()
  val form = remember(entityId, normalizedInitialIcon, normalizedInitialColor) {
    FolderIconPickerForm(
      scope = scope,
      initialIconName = normalizedInitialIcon,
      initialColor = normalizedInitialColor,
    )
  }

  var isUpdating by remember { mutableStateOf(false) }

  fun updateSelection(nextIconName: String, nextColor: String) {
    if (isUpdating) {
      return
    }

    if (form.iconName.initialValue == nextIconName && form.color.initialValue == nextColor) {
      return
    }

    form.iconName.setValue(nextIconName)
    form.color.setValue(nextColor)
    isUpdating = true

    model.updateEntityIcon(
      entityId = entityId,
      icon = nextIconName,
      iconColor = nextColor,
    ) { success ->
      if (success) {
        form.commit()
        onUpdated()
      } else {
        form.rollback()
      }
      isUpdating = false
    }
  }

  val currentTint = resolveEntityIconTint(form.color.value, AppTheme.colors) ?: AppTheme.colors.textSecondary
  val iconGridScrollState = rememberScrollState()

  BottomSheetScaffold(
    title = "아이콘 변경",
    applyContentImeOrNavigationBarsPadding = false,
    leadingAction = {
      BottomSheetHeaderTextAction(
        text = "완료",
        color = AppTheme.colors.brand,
        textStyle = AppTheme.typography.action.copy(fontWeight = FontWeight.W700),
        enabled = !isUpdating,
        onClick = { dismiss() },
      )
    },
  ) {
    Column(
      verticalArrangement = Arrangement.spacedBy(12.dp),
    ) {
      IconColorRow(
        colors = entityIconColors,
        selectedColor = form.color.value,
        enabled = !isUpdating,
        onColorSelect = { nextColor -> updateSelection(form.iconName.value, nextColor) },
      )

      Box(modifier = Modifier.fillMaxWidth()) {
        BoxWithConstraints(
          modifier = Modifier
            .fillMaxWidth()
            .heightIn(max = FolderIconPickerGridMaxHeight)
            .verticalScroll(iconGridScrollState),
        ) {
          val cellSize = ((maxWidth - FolderIconPickerCellSpacing * 6) / 7).let { size ->
            if (size < 32.dp) 32.dp else size
          }

          Column(
            modifier = Modifier.safeBottomPadding(FolderIconPickerGridBottomInset),
            verticalArrangement = Arrangement.spacedBy(FolderIconPickerCellSpacing),
          ) {
            entityIcons.chunked(7).forEach { row ->
              Row(
                horizontalArrangement = Arrangement.spacedBy(FolderIconPickerCellSpacing),
                modifier = Modifier.fillMaxWidth(),
              ) {
                row.forEach { icon ->
                  IconGridCell(
                    icon = icon,
                    tint = currentTint,
                    selected = form.iconName.value == icon.name,
                    enabled = !isUpdating,
                    size = cellSize,
                    onSelect = { updateSelection(icon.name, form.color.value) },
                  )
                }
              }
            }
          }
        }

        if (iconGridScrollState.value > 0) {
          Box(
            modifier = Modifier
              .align(Alignment.TopCenter)
              .fillMaxWidth()
              .heightIn(min = FolderIconPickerTopFadeHeight, max = FolderIconPickerTopFadeHeight)
              .background(
                Brush.verticalGradient(
                  colors = listOf(
                    AppTheme.colors.surfaceRaised,
                    AppTheme.colors.surfaceRaised.copy(alpha = 0f),
                  ),
                ),
              ),
          )
        }
      }
    }
  }
}

@Composable
private fun IconColorRow(
  colors: List<EntityIconColorOption>,
  selectedColor: String,
  enabled: Boolean,
  onColorSelect: (String) -> Unit,
) {
  Row(
    modifier = Modifier.fillMaxWidth(),
    horizontalArrangement = Arrangement.SpaceEvenly,
    verticalAlignment = Alignment.CenterVertically,
  ) {
    colors.forEach { color ->
      IconColorChip(
        color = color,
        selected = selectedColor == color.value,
        enabled = enabled,
        onClick = { onColorSelect(color.value) },
      )
    }
  }
}

@Composable
private fun IconColorChip(
  color: EntityIconColorOption,
  selected: Boolean,
  enabled: Boolean,
  onClick: () -> Unit,
) {
  InteractionScope {
    Box(
      contentAlignment = Alignment.Center,
      modifier = Modifier
        .size(20.dp)
        .clip(CircleShape)
        .background(color.color, CircleShape)
        .border(
          width = if (selected) 1.dp else 0.dp,
          color = if (selected) AppTheme.colors.borderStrong else Color.Transparent,
          shape = CircleShape,
        )
        .clickable(enabled = enabled) { onClick() }
        .pressScale(0.96f),
    ) {
      if (selected) {
        Icon(
          icon = Lucide.Check,
          modifier = Modifier.size(10.dp),
          tint = Color.White,
        )
      }
    }
  }
}

@Composable
private fun IconGridCell(
  icon: EntityIconOption,
  tint: Color,
  selected: Boolean,
  enabled: Boolean,
  size: androidx.compose.ui.unit.Dp,
  onSelect: () -> Unit,
) {
  InteractionScope {
    Box(
      contentAlignment = Alignment.Center,
      modifier = Modifier
        .size(size)
        .clip(RoundedCornerShape(4.dp))
        .background(if (selected) AppTheme.colors.surfaceSunken else Color.Transparent)
        .clickable(enabled = enabled) { onSelect() }
        .pressScale(0.98f),
    ) {
      Icon(
        icon = icon.icon,
        modifier = Modifier.size(FolderIconPickerGridIconSize),
        tint = tint,
      )

      if (selected) {
        Box(
          modifier = Modifier
            .align(Alignment.BottomCenter)
            .padding(bottom = FolderIconPickerSelectionDotBottomInset)
            .size(FolderIconPickerSelectionDotSize)
            .clip(CircleShape)
            .background(tint, CircleShape),
        )
      }
    }
  }
}
