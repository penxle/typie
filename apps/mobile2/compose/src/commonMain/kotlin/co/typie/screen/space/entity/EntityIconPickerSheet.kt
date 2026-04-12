package co.typie.screen.space.entity

import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.gestures.Orientation
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.WindowInsets
import androidx.compose.foundation.layout.WindowInsetsSides
import androidx.compose.foundation.layout.asPaddingValues
import androidx.compose.foundation.layout.aspectRatio
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.only
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.safeDrawing
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.lazy.grid.GridCells
import androidx.compose.foundation.lazy.grid.LazyVerticalGrid
import androidx.compose.foundation.lazy.grid.items
import androidx.compose.foundation.lazy.grid.rememberLazyGridState
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
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
import co.typie.ext.desktopDragScroll
import co.typie.ext.pressScale
import co.typie.form.FormState
import co.typie.icons.Lucide
import co.typie.overlay.LocalToast
import co.typie.result.Result
import co.typie.result.onException
import co.typie.result.onOk
import co.typie.result.withDefaultExceptionHandler
import co.typie.ui.EntityIconColorOption
import co.typie.ui.EntityIconOption
import co.typie.ui.component.sheet.ActionHeader
import co.typie.ui.component.sheet.HeaderTextAction
import co.typie.ui.component.sheet.SheetChrome
import co.typie.ui.component.sheet.SheetCollapsePolicy
import co.typie.ui.component.sheet.SheetDetent
import co.typie.ui.component.sheet.SheetDragDismissBehavior
import co.typie.ui.component.sheet.SheetHapticPolicy
import co.typie.ui.component.sheet.SheetInsetPolicy
import co.typie.ui.component.sheet.SheetLayout
import co.typie.ui.component.sheet.SheetMode
import co.typie.ui.component.sheet.SheetOverlaySpec
import co.typie.ui.component.sheet.SheetPadding
import co.typie.ui.component.sheet.SheetPresentation
import co.typie.ui.component.sheet.SheetSizePolicy
import co.typie.ui.component.sheet.dismiss
import co.typie.ui.component.sheet.rememberSheetBoundaryHandoffFlingBehavior
import co.typie.ui.component.sheet.sheetDragRegion
import co.typie.ui.component.sheet.sheetPresentation
import co.typie.ui.entityIconColors
import co.typie.ui.entityIcons
import co.typie.ui.icon.Icon
import co.typie.ui.resolveEntityIconTint
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
private val EntityIconPickerCollapsedDetent = SheetDetent.Fixed(EntityIconPickerCollapsedHeight)
private val EntityIconPickerExpandedDetent = SheetDetent.TopGap(EntityIconPickerExpandedTopGap)

internal interface EntityIconSheetModel {
  suspend fun updateEntityIcon(
    entityId: String,
    icon: String,
    iconColor: String,
  ): Result<Unit, Nothing>
}

internal fun entityIconPickerSheet(
  model: EntityIconSheetModel,
  entityId: String,
  initialIcon: String?,
  initialColor: String?,
  defaultIconName: String,
  onUpdated: () -> Unit = {},
): SheetPresentation<Unit> =
  sheetPresentation(
    spec =
      SheetOverlaySpec(
        mode = SheetMode.Modal,
        sizePolicy =
          SheetSizePolicy.Detents(
            initial = EntityIconPickerCollapsedDetent,
            available = listOf(EntityIconPickerCollapsedDetent, EntityIconPickerExpandedDetent),
            collapsePolicy = SheetCollapsePolicy.ProgrammaticOnly,
            dragDismissBehavior = SheetDragDismissBehavior.FromCurrentDetent,
          ),
        chrome = SheetChrome.Default,
        haptics = SheetHapticPolicy(onPresent = true, onDetentSnap = true),
      )
  ) {
    val normalizedInitialIcon = initialIcon?.trim()?.takeIf { it.isNotEmpty() } ?: defaultIconName
    val normalizedInitialColor =
      initialColor?.trim()?.takeIf { it.isNotEmpty() } ?: DEFAULT_ENTITY_ICON_COLOR
    val toast = LocalToast.current
    val scope = rememberCoroutineScope()
    val form =
      remember(entityId, normalizedInitialIcon, normalizedInitialColor) {
        EntityIconPickerForm(
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

      scope.launch {
        model
          .updateEntityIcon(entityId = entityId, icon = nextIconName, iconColor = nextColor)
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
      resolveEntityIconTint(form.color.value, AppTheme.colors) ?: AppTheme.colors.textSecondary
    val iconGridState = rememberLazyGridState()
    val iconGridFlingBehavior =
      rememberSheetBoundaryHandoffFlingBehavior(
        isAtSheetDismissBoundary = { !iconGridState.canScrollBackward }
      )

    SheetLayout(
      fillHeight = true,
      bodyScroll = false,
      bodyInsetPolicy = SheetInsetPolicy.None,
      padding =
        SheetPadding(
          header = PaddingValues(horizontal = 16.dp),
          body = PaddingValues(horizontal = 16.dp),
        ),
      header = {
        ActionHeader(
          title = "아이콘 변경",
          leading = {
            HeaderTextAction(
              text = "완료",
              color = AppTheme.colors.brand,
              textStyle = AppTheme.typography.action.copy(fontWeight = FontWeight.W700),
              enabled = !isUpdating,
              onClick = { dismiss() },
            )
          },
        )
      },
    ) {
      Column(modifier = Modifier.fillMaxSize(), verticalArrangement = Arrangement.spacedBy(12.dp)) {
        IconColorRow(
          colors = entityIconColors,
          selectedColor = form.color.value,
          enabled = !isUpdating,
          modifier = Modifier.sheetDragRegion(),
          onColorSelect = { nextColor -> updateSelection(form.iconName.value, nextColor) },
        )

        Box(modifier = Modifier.fillMaxWidth().weight(1f)) {
          val safeBottom =
            WindowInsets.safeDrawing
              .only(WindowInsetsSides.Bottom)
              .asPaddingValues()
              .calculateBottomPadding()

          LazyVerticalGrid(
            columns = GridCells.Fixed(7),
            state = iconGridState,
            flingBehavior = iconGridFlingBehavior,
            modifier =
              Modifier.fillMaxSize()
                .desktopDragScroll(state = iconGridState, orientation = Orientation.Vertical),
            contentPadding = PaddingValues(bottom = EntityIconPickerGridBottomInset + safeBottom),
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

          if (iconGridState.canScrollBackward) {
            Box(
              modifier =
                Modifier.align(Alignment.TopCenter)
                  .fillMaxWidth()
                  .height(EntityIconPickerTopFadeHeight)
                  .background(
                    Brush.verticalGradient(
                      colors =
                        listOf(
                          AppTheme.colors.surfaceRaised,
                          AppTheme.colors.surfaceRaised.copy(alpha = 0f),
                        )
                    )
                  )
            )
          }
        }
      }
    }
  }

private class EntityIconPickerForm(
  scope: CoroutineScope,
  initialIconName: String,
  initialColor: String,
) : FormState(scope) {
  val iconName = field(initialIconName) { focusable = false }
  val color = field(initialColor) { focusable = false }
}

@Composable
private fun IconColorRow(
  colors: List<EntityIconColorOption>,
  selectedColor: String,
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
          .clip(RoundedCornerShape(4.dp))
          .background(if (selected) AppTheme.colors.surfaceSunken else Color.Transparent)
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
              .clip(CircleShape)
              .background(tint, CircleShape)
        )
      }
    }
  }
}
