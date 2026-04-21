package co.typie.domain.entity

import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.WindowInsets
import androidx.compose.foundation.layout.asPaddingValues
import androidx.compose.foundation.layout.aspectRatio
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.lazy.grid.GridCells
import androidx.compose.foundation.lazy.grid.LazyVerticalGrid
import androidx.compose.foundation.lazy.grid.items
import androidx.compose.foundation.lazy.grid.rememberLazyGridState
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import co.typie.ext.InteractionScope
import co.typie.ext.clickable
import co.typie.ext.pressScale
import co.typie.ext.safeDrawing
import co.typie.form.FormState
import co.typie.icons.Lucide
import co.typie.result.Result
import co.typie.result.onException
import co.typie.result.onOk
import co.typie.result.withDefaultExceptionHandler
import co.typie.ui.EntityIconColorOption
import co.typie.ui.EntityIconOption
import co.typie.ui.component.Text
import co.typie.ui.component.scrollFog
import co.typie.ui.component.sheet.SheetBar
import co.typie.ui.component.sheet.SheetBarTextButton
import co.typie.ui.component.sheet.SheetLayout
import co.typie.ui.component.sheet.SheetPadding
import co.typie.ui.component.sheet.SheetScope
import co.typie.ui.component.sheet.SheetStop
import co.typie.ui.component.sheet.dismiss
import co.typie.ui.component.toast.LocalToast
import co.typie.ui.entityIcons
import co.typie.ui.icon.Icon
import co.typie.ui.rememberEntityIconColorOptions
import co.typie.ui.resolveEntityIconTint
import co.typie.ui.theme.AppShapes
import co.typie.ui.theme.AppTheme
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.launch

private const val DEFAULT_ENTITY_ICON_COLOR = "gray"
private val EntityIconPickerCellSpacing = 2.dp
private val EntityIconPickerGridIconSize = 18.dp
private val EntityIconPickerCollapsedHeight = 360.dp
private val EntityIconPickerExpandedTopGap = 128.dp
private val EntityIconPickerSelectionDotSize = 4.dp
private val EntityIconPickerSelectionDotBottomInset = 4.dp
private val EntityIconPickerGridBottomInset = 12.dp
private val EntityIconPickerTopFadeHeight = 16.dp
internal val EntityIconPickerStops =
  listOf(
    SheetStop.Bottom(EntityIconPickerCollapsedHeight),
    SheetStop.Top(EntityIconPickerExpandedTopGap),
  )
internal val EntityIconPickerStopPolicy = SheetStop.Policy.DismissFromTopStop

internal interface EntityIconPickerSheetModel {
  suspend fun updateEntityIcons(
    entityIds: List<String>,
    icon: String?,
    iconColor: String?,
  ): Result<Unit, Nothing>
}

@Composable
context(_: SheetScope<Unit>)
internal fun EntityIconPickerSheet(
  model: EntityIconPickerSheetModel,
  entityId: String,
  initialIcon: String?,
  initialColor: String?,
  defaultIconName: String,
  onUpdated: () -> Unit = {},
) {
  EntityIconPickerSheet(
    model = model,
    entityIds = listOf(entityId),
    initialIcon = initialIcon,
    initialColor = initialColor,
    defaultIconName = defaultIconName,
    onUpdated = onUpdated,
  )
}

@Composable
context(_: SheetScope<Unit>)
internal fun EntityIconPickerSheet(
  model: EntityIconPickerSheetModel,
  entityIds: List<String>,
  initialIcon: String?,
  initialColor: String?,
  defaultIconName: String? = null,
  onUpdated: () -> Unit = {},
) {
  val normalizedInitialIcon = initialIcon?.trim()?.takeIf { it.isNotEmpty() } ?: defaultIconName
  val normalizedInitialColor =
    initialColor?.trim()?.takeIf { it.isNotEmpty() }
      ?: defaultIconName?.let { DEFAULT_ENTITY_ICON_COLOR }
  val toast = LocalToast.current
  val scope = rememberCoroutineScope()
  val form =
    remember(entityIds, normalizedInitialIcon, normalizedInitialColor) {
      EntityIconPickerForm(
        scope = scope,
        initialIconName = normalizedInitialIcon,
        initialColor = normalizedInitialColor,
      )
    }

  var isUpdating by remember { mutableStateOf(false) }

  fun updateSelection(nextIconName: String?, nextColor: String?) {
    if (isUpdating || (nextIconName == null && nextColor == null)) {
      return
    }

    if (form.iconName.value == nextIconName && form.color.value == nextColor) {
      return
    }

    form.iconName.setValue(nextIconName)
    form.color.setValue(nextColor)
    isUpdating = true

    scope.launch {
      model
        .updateEntityIcons(entityIds = entityIds, icon = nextIconName, iconColor = nextColor)
        .withDefaultExceptionHandler(toast)
        .onOk {
          form.commit()
          onUpdated()
        }
        .onException { form.rollback() }
      isUpdating = false
    }
  }

  val currentTint =
    form.color.value?.let { resolveEntityIconTint(it, AppTheme.colors) }
      ?: AppTheme.colors.textMuted
  val iconGridState = rememberLazyGridState()

  SheetLayout(
    fillHeight = true,
    bodyScroll = false,
    padding =
      SheetPadding(
        header = PaddingValues(horizontal = 16.dp),
        body = PaddingValues(horizontal = 16.dp),
      ),
    header = {
      SheetBar(
        leading = {
          SheetBarTextButton(
            text = "완료",
            color = AppTheme.colors.textDefault,
            enabled = !isUpdating,
            onClick = { dismiss() },
          )
        },
        center = {
          Text(
            text = "아이콘 변경",
            style = AppTheme.typography.title,
            color = AppTheme.colors.textDefault,
            overflow = TextOverflow.Ellipsis,
            maxLines = 1,
          )
        },
      )
    },
  ) {
    Column(modifier = Modifier.fillMaxSize(), verticalArrangement = Arrangement.spacedBy(6.dp)) {
      IconColorRow(
        colors = rememberEntityIconColorOptions(),
        selectedColor = form.color.value,
        enabled = !isUpdating,
        modifier = Modifier,
        onColorSelect = { nextColor -> updateSelection(form.iconName.value, nextColor) },
      )

      val gridFogInsets = remember { PaddingValues(top = EntityIconPickerTopFadeHeight) }

      Box(
        modifier =
          Modifier.fillMaxWidth()
            .weight(1f)
            .scrollFog(padding = gridFogInsets, color = AppTheme.colors.surfaceCanvas)
      ) {
        val safeBottom = WindowInsets.safeDrawing.asPaddingValues().calculateBottomPadding()

        LazyVerticalGrid(
          columns = GridCells.Fixed(7),
          state = iconGridState,
          modifier = Modifier.fillMaxSize(),
          contentPadding =
            PaddingValues(
              top = gridFogInsets.calculateTopPadding(),
              bottom = EntityIconPickerGridBottomInset + safeBottom,
            ),
          horizontalArrangement = Arrangement.spacedBy(EntityIconPickerCellSpacing),
          verticalArrangement = Arrangement.spacedBy(EntityIconPickerCellSpacing),
        ) {
          items(entityIcons, key = { it.name }) { icon ->
            IconGridCell(
              icon = icon,
              tint = currentTint,
              selected = form.iconName.value == icon.name,
              enabled = !isUpdating,
              onSelect = { updateSelection(icon.name, form.color.value) },
            )
          }
        }
      }
    }
  }
}

private class EntityIconPickerForm(
  scope: CoroutineScope,
  initialIconName: String?,
  initialColor: String?,
) : FormState(scope) {
  val iconName = field(initialIconName) { focusable = false }
  val color = field(initialColor) { focusable = false }
}

@Composable
private fun IconColorRow(
  colors: List<EntityIconColorOption>,
  selectedColor: String?,
  enabled: Boolean,
  modifier: Modifier = Modifier,
  onColorSelect: (String) -> Unit,
) {
  Row(
    modifier = modifier.fillMaxWidth(),
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
      modifier =
        Modifier.size(20.dp)
          .clip(AppShapes.circle)
          .background(color.color, AppShapes.circle)
          .border(
            width = if (selected) 1.dp else 0.dp,
            color = if (selected) AppTheme.colors.borderEmphasis else Color.Transparent,
            shape = AppShapes.circle,
          )
          .clickable(enabled = enabled) { onClick() }
          .pressScale(0.96f),
    ) {
      if (selected) {
        Icon(icon = Lucide.Check, modifier = Modifier.size(10.dp), tint = Color.White)
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
  onSelect: () -> Unit,
) {
  InteractionScope {
    Box(
      contentAlignment = Alignment.Center,
      modifier =
        Modifier.fillMaxWidth()
          .aspectRatio(1f)
          .clip(AppShapes.rounded(AppShapes.sm))
          .background(if (selected) AppTheme.colors.surfaceInset else Color.Transparent)
          .clickable(enabled = enabled) { onSelect() }
          .pressScale(0.98f),
    ) {
      Icon(icon = icon.icon, modifier = Modifier.size(EntityIconPickerGridIconSize), tint = tint)

      if (selected) {
        Box(
          modifier =
            Modifier.align(Alignment.BottomCenter)
              .padding(bottom = EntityIconPickerSelectionDotBottomInset)
              .size(EntityIconPickerSelectionDotSize)
              .clip(AppShapes.circle)
              .background(tint, AppShapes.circle)
        )
      }
    }
  }
}
