package co.typie.screen.space.folder

import androidx.compose.foundation.background
import androidx.compose.foundation.border
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
import co.typie.ext.pressScale
import co.typie.form.FormState
import co.typie.icons.Lucide
import co.typie.overlay.LocalToast
import co.typie.result.onException
import co.typie.result.onOk
import co.typie.result.withDefaultExceptionHandler
import co.typie.ui.EntityIconColorOption
import co.typie.ui.EntityIconOption
import co.typie.ui.component.sheet.ActionHeader
import co.typie.ui.component.sheet.HeaderTextAction
import co.typie.ui.component.sheet.SheetChrome
import co.typie.ui.component.sheet.SheetDetent
import co.typie.ui.component.sheet.SheetDragDismissBehavior
import co.typie.ui.component.sheet.SheetHapticPolicy
import co.typie.ui.component.sheet.SheetInsetPolicy
import co.typie.ui.component.sheet.SheetLayout
import co.typie.ui.component.sheet.SheetMode
import co.typie.ui.component.sheet.SheetOverlaySpec
import co.typie.ui.component.sheet.SheetPadding
import co.typie.ui.component.sheet.SheetPresentation
import co.typie.ui.component.sheet.SheetScope
import co.typie.ui.component.sheet.SheetSizePolicy
import co.typie.ui.component.sheet.sheetPresentation
import co.typie.ui.entityIconColors
import co.typie.ui.entityIcons
import co.typie.ui.icon.Icon
import co.typie.ui.resolveEntityIconTint
import co.typie.ui.theme.AppTheme
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.launch

private const val DEFAULT_ENTITY_ICON_NAME = "folder"
private const val DEFAULT_ENTITY_ICON_COLOR = "gray"
private val FolderIconPickerCellSpacing = 2.dp
private val FolderIconPickerGridIconSize = 18.dp
private val FolderIconPickerCollapsedHeight = 360.dp
private val FolderIconPickerExpandedTopGap = 128.dp
private val FolderIconPickerSelectionDotSize = 4.dp
private val FolderIconPickerSelectionDotBottomInset = 4.dp
private val FolderIconPickerGridBottomInset = 12.dp
private val FolderIconPickerTopFadeHeight = 16.dp
private val FolderIconPickerCollapsedDetent = SheetDetent.Fixed(FolderIconPickerCollapsedHeight)
private val FolderIconPickerExpandedDetent = SheetDetent.TopGap(FolderIconPickerExpandedTopGap)

internal fun folderIconPickerSheetSpec(): SheetOverlaySpec {
  return SheetOverlaySpec(
    mode = SheetMode.Modal,
    sizePolicy =
      SheetSizePolicy.Detents(
        initial = FolderIconPickerCollapsedDetent,
        available = listOf(FolderIconPickerCollapsedDetent, FolderIconPickerExpandedDetent),
        dragDismissBehavior = SheetDragDismissBehavior.FromCurrentDetent,
      ),
    chrome = SheetChrome.Default,
    haptics = SheetHapticPolicy(onPresent = true, onDetentSnap = true),
  )
}

internal fun folderIconPickerSheet(
  model: FolderViewModel,
  entityId: String,
  initialIcon: String?,
  initialColor: String?,
  onUpdated: () -> Unit = {},
): SheetPresentation<Unit> =
  sheetPresentation(spec = folderIconPickerSheetSpec()) {
    FolderIconPickerSheetContent(
      model = model,
      entityId = entityId,
      initialIcon = initialIcon,
      initialColor = initialColor,
      onUpdated = onUpdated,
    )
  }

private class FolderIconPickerForm(
  scope: CoroutineScope,
  initialIconName: String,
  initialColor: String,
) : FormState(scope) {
  val iconName = field(initialIconName) { focusable = false }
  val color = field(initialColor) { focusable = false }
}

@Composable
private fun SheetScope<Unit>.FolderIconPickerSheetContent(
  model: FolderViewModel,
  entityId: String,
  initialIcon: String?,
  initialColor: String?,
  onUpdated: () -> Unit = {},
) {
  val normalizedInitialIcon =
    initialIcon?.trim()?.takeIf { it.isNotEmpty() } ?: DEFAULT_ENTITY_ICON_NAME
  val normalizedInitialColor =
    initialColor?.trim()?.takeIf { it.isNotEmpty() } ?: DEFAULT_ENTITY_ICON_COLOR
  val toast = LocalToast.current
  val scope = rememberCoroutineScope()
  val form =
    remember(entityId, normalizedInitialIcon, normalizedInitialColor) {
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
          modifier = Modifier.fillMaxSize(),
          contentPadding = PaddingValues(bottom = FolderIconPickerGridBottomInset + safeBottom),
          horizontalArrangement = Arrangement.spacedBy(FolderIconPickerCellSpacing),
          verticalArrangement = Arrangement.spacedBy(FolderIconPickerCellSpacing),
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
                .height(FolderIconPickerTopFadeHeight)
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
      Icon(icon = icon.icon, modifier = Modifier.size(FolderIconPickerGridIconSize), tint = tint)

      if (selected) {
        Box(
          modifier =
            Modifier.align(Alignment.BottomCenter)
              .padding(bottom = FolderIconPickerSelectionDotBottomInset)
              .size(FolderIconPickerSelectionDotSize)
              .clip(CircleShape)
              .background(tint, CircleShape)
        )
      }
    }
  }
}
