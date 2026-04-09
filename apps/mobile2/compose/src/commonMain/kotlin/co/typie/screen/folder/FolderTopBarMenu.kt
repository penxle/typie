package co.typie.screen.folder

import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.offset
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.layout.widthIn
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableIntStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.layout.onSizeChanged
import androidx.compose.ui.platform.LocalDensity
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.IntOffset
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import co.typie.ext.InteractionScope
import co.typie.ext.clickable
import co.typie.ext.pressScale
import co.typie.graphql.type.EntityAvailability
import co.typie.graphql.type.EntityVisibility
import co.typie.icons.Lucide
import co.typie.screen.home.resolveEntityIconAppearance
import co.typie.ui.component.CardDivider
import co.typie.ui.component.ResponsiveContainerDefaults
import co.typie.ui.component.Text
import co.typie.ui.component.popover.LocalPopoverPaneTransition
import co.typie.ui.component.popover.Popover
import co.typie.ui.component.popover.PopoverDefaults
import co.typie.ui.component.popover.PopoverList
import co.typie.ui.component.popover.PopoverListItem
import co.typie.ui.component.popover.PopoverPlacement
import co.typie.ui.component.popover.PopoverScope
import co.typie.ui.component.popover.PopoverTransitionElement
import co.typie.ui.component.popover.PopoverTransitionFrame
import co.typie.ui.component.topbar.TopBarDefaults
import co.typie.ui.icon.Icon
import co.typie.ui.shape.SquircleShape
import co.typie.ui.theme.AppTheme
import kotlin.math.max
import kotlin.math.roundToInt

private val FolderTopBarVerticalOffset = (TopBarDefaults.Height - TopBarDefaults.ButtonSize) / 2
private val FolderTopBarCollapsedRadius = PopoverDefaults.ExpandedRadius
internal val FolderTopBarPopoverScreenPadding = PaddingValues(
  start = TopBarDefaults.HorizontalPadding,
  top = FolderTopBarVerticalOffset,
  end = TopBarDefaults.HorizontalPadding,
  bottom = FolderTopBarVerticalOffset + 100.dp,
)
private val FolderPaneHeaderTopHeight = 28.dp
private val FolderPaneHeaderIconTargetLeft = 16.dp
private val FolderPaneHeaderIconTargetSize = 20.dp
private val FolderPaneHeaderTitleGap = 12.dp
private val FolderPaneHeaderSourceHorizontalInset = 14.dp
private val FolderPaneHeaderSourceIconSize = 18.dp
private val FolderPaneHeaderSourceIconGap = 10.dp
private val FolderPaneHeaderTextLeft = FolderPaneHeaderIconTargetLeft + FolderPaneHeaderIconTargetSize + FolderPaneHeaderTitleGap
private val FolderPaneHeaderCloseButtonSize = 44.dp
private val FolderPaneHeaderCloseButtonVisualSize = 24.dp
private val FolderPaneHeaderCloseButtonIconSize = 16.dp
private val FolderPaneHeaderCloseButtonEndInset = 6.dp
private val FolderPaneHeaderCloseButtonGap = 8.dp

@Composable
internal fun FolderTopBarCenterMenu(
  title: String,
  subtitle: String,
  breadcrumbNames: List<String>,
  visibilityName: EntityVisibility?,
  availabilityName: EntityAvailability?,
  iconName: String?,
  iconColor: String?,
  onAction: (FolderAction, closePopover: () -> Unit) -> Unit,
  modifier: Modifier = Modifier,
) {
  Popover(
    placement = PopoverPlacement.BelowCenter,
    screenPadding = FolderTopBarPopoverScreenPadding,
    collapsedCornerRadius = FolderTopBarCollapsedRadius,
    maxWidth = ResponsiveContainerDefaults.MaxWidth,
    minWidth = 280.dp,
    expandToMaxWidth = true,
    anchor = {
      FolderTopBarCapsule(
        title = title,
        subtitle = subtitle,
        iconName = iconName,
        iconColor = iconColor,
        modifier = modifier,
      )
    },
    pane = {
      FolderTopBarCenterPane(
        title = title,
        subtitle = subtitle,
        breadcrumbNames = breadcrumbNames,
        visibilityName = visibilityName,
        availabilityName = availabilityName,
        iconName = iconName,
        iconColor = iconColor,
        onAction = onAction,
      )
    },
  )
}

@Composable
internal fun PopoverScope.FolderTopBarCenterPane(
  title: String,
  subtitle: String,
  breadcrumbNames: List<String>,
  visibilityName: EntityVisibility?,
  availabilityName: EntityAvailability?,
  iconName: String?,
  iconColor: String?,
  onAction: (FolderAction, closePopover: () -> Unit) -> Unit,
) {
  Column(
    modifier = Modifier
      .padding(
        start = PopoverDefaults.PanePadding,
        top = PopoverDefaults.PanePadding + 4.dp,
        end = PopoverDefaults.PanePadding,
        bottom = PopoverDefaults.PanePadding,
      )
      .fillMaxWidth()
      .widthIn(min = 280.dp, max = ResponsiveContainerDefaults.MaxWidth),
  ) {
      FolderTopBarPaneHeader(
        title = title,
        subtitle = subtitle,
        breadcrumbNames = breadcrumbNames,
        visibilityName = visibilityName,
      availabilityName = availabilityName,
      iconName = iconName,
      iconColor = iconColor,
      onClose = { close() },
    )

    Spacer(Modifier.height(6.dp))

    FolderTopBarActionList(
      onAction = onAction,
    )
  }
}

@Composable
internal fun FolderTopBarCapsule(
  title: String,
  subtitle: String,
  iconName: String?,
  iconColor: String?,
  modifier: Modifier = Modifier,
) {
  val shape = SquircleShape(FolderTopBarCollapsedRadius)
  val entityIcon = resolveEntityIconAppearance(
    iconName = iconName,
    iconColor = iconColor,
    fallbackIcon = Lucide.Folder,
    fallbackTint = AppTheme.colors.textSecondary,
    colors = AppTheme.colors,
  )

  Row(
    verticalAlignment = Alignment.CenterVertically,
    modifier = modifier
      .height(TopBarDefaults.TitleHeight)
      .then(TopBarDefaults.controlShadowModifier(shape))
      .clip(shape)
      .background(TopBarDefaults.controlBackgroundColor(), shape)
      .border(1.dp, TopBarDefaults.controlBorderColor(), shape)
      .padding(horizontal = 14.dp),
  ) {
    Icon(
      icon = entityIcon.icon,
      modifier = Modifier.size(TopBarDefaults.TitleIconSize),
      tint = entityIcon.tint,
    )

    Spacer(Modifier.width(TopBarDefaults.TitleIconGap))

    Column(modifier = Modifier.weight(1f)) {
      Text(
        text = title,
        style = AppTheme.typography.title.copy(fontSize = 14.sp),
        maxLines = 1,
        overflow = TextOverflow.Ellipsis,
      )
      Spacer(Modifier.height(1.dp))
      Text(
        text = subtitle,
        style = AppTheme.typography.caption.copy(fontSize = 11.sp),
        color = AppTheme.colors.textTertiary,
        maxLines = 1,
        overflow = TextOverflow.Ellipsis,
      )
    }
  }
}

@Composable
private fun FolderTopBarPaneHeader(
  title: String,
  subtitle: String,
  breadcrumbNames: List<String>,
  visibilityName: EntityVisibility?,
  availabilityName: EntityAvailability?,
  iconName: String?,
  iconColor: String?,
  onClose: () -> Unit,
) {
  val entityIcon = resolveEntityIconAppearance(
    iconName = iconName,
    iconColor = iconColor,
    fallbackIcon = Lucide.Folder,
    fallbackTint = AppTheme.colors.textSecondary,
    colors = AppTheme.colors,
  )
  val visibility = folderVisibilityPresentation(
    visibility = visibilityName,
    availability = availabilityName,
  )

  Column(
    modifier = Modifier.fillMaxWidth(),
  ) {
    Box(
      modifier = Modifier
        .fillMaxWidth()
        .height(FolderPaneHeaderTopHeight),
    ) {
      PopoverTransitionElement(
        collapsedFrame = PopoverTransitionFrame(
          left = FolderPaneHeaderSourceHorizontalInset,
          top = (TopBarDefaults.ButtonSize - FolderPaneHeaderSourceIconSize) / 2,
          width = FolderPaneHeaderSourceIconSize,
          height = FolderPaneHeaderSourceIconSize,
        ),
        expandedFrame = PopoverTransitionFrame(
          left = FolderPaneHeaderIconTargetLeft,
          top = (FolderPaneHeaderTopHeight - FolderPaneHeaderIconTargetSize) / 2,
          width = FolderPaneHeaderIconTargetSize,
          height = FolderPaneHeaderIconTargetSize,
        ),
      ) {
        Icon(
          icon = entityIcon.icon,
          modifier = Modifier.size(FolderPaneHeaderIconTargetSize),
          tint = entityIcon.tint,
        )
      }

      FolderTopBarTransitionTitle(title = title)

      FolderTopBarCloseButton(
        onClick = onClose,
        modifier = Modifier
          .align(Alignment.CenterEnd)
          .padding(end = FolderPaneHeaderCloseButtonEndInset),
      )
    }

    Spacer(Modifier.height(4.dp))

    Column(
      modifier = Modifier.padding(start = FolderPaneHeaderTextLeft, end = 16.dp),
    ) {
      FolderTopBarBreadcrumbs(names = breadcrumbNames)

      Spacer(Modifier.height(4.dp))

      Text(
        text = visibility.label,
        style = AppTheme.typography.caption.copy(fontSize = 14.sp),
        color = if (visibility.isShared) AppTheme.colors.brand else AppTheme.colors.textMuted,
      )

      Text(
        text = subtitle,
        style = AppTheme.typography.caption.copy(fontSize = 14.sp),
        color = AppTheme.colors.textMuted,
      )
    }
  }
}

@Composable
private fun FolderTopBarBreadcrumbs(
  names: List<String>,
) {
  Row(
    modifier = Modifier.fillMaxWidth(),
    horizontalArrangement = Arrangement.spacedBy(4.dp),
    verticalAlignment = Alignment.CenterVertically,
  ) {
    names.forEachIndexed { index, name ->
      if (index == 0) {
        Text(
          text = name,
          modifier = Modifier.weight(1f, fill = false),
          style = AppTheme.typography.caption.copy(fontSize = 14.sp),
          color = AppTheme.colors.textTertiary,
          maxLines = 1,
          overflow = TextOverflow.Ellipsis,
        )
      } else {
        Row(
          modifier = Modifier.weight(1f, fill = false),
          horizontalArrangement = Arrangement.spacedBy(4.dp),
          verticalAlignment = Alignment.CenterVertically,
        ) {
          Icon(
            icon = Lucide.ChevronRight,
            modifier = Modifier.size(14.dp),
            tint = AppTheme.colors.textTertiary,
          )

          Text(
            text = name,
            modifier = Modifier.weight(1f, fill = false),
            style = AppTheme.typography.caption.copy(fontSize = 14.sp),
            color = AppTheme.colors.textTertiary,
            maxLines = 1,
            overflow = TextOverflow.Ellipsis,
          )
        }
      }
    }
  }
}

@Composable
private fun FolderTopBarTransitionTitle(
  title: String,
) {
  val density = LocalDensity.current
  val transition = LocalPopoverPaneTransition.current
  val progress = (transition?.progress ?: 1f).coerceIn(0f, 1f)
  val anchorContentRect = transition?.anchorContentRect
  var paneWidthPx by remember { mutableIntStateOf(0) }

  Box(
    modifier = Modifier
      .fillMaxWidth()
      .onSizeChanged { paneWidthPx = it.width }
      .height(FolderPaneHeaderTopHeight),
  ) {
    val resolvedPaneWidthPx = if (paneWidthPx > 0) {
      paneWidthPx.toFloat()
    } else {
      max(anchorContentRect?.width ?: 0f, with(density) { 280.dp.toPx() })
    }
    val horizontalInsetPx = with(density) { FolderPaneHeaderSourceHorizontalInset.toPx() }
    val sourceIconSizePx = with(density) { FolderPaneHeaderSourceIconSize.toPx() }
    val sourceIconGapPx = with(density) { FolderPaneHeaderSourceIconGap.toPx() }
    val targetTextLeftPx = with(density) { FolderPaneHeaderTextLeft.toPx() }
    val trailingReservedWidthPx = with(density) {
      FolderPaneHeaderCloseButtonEndInset.toPx() +
        FolderPaneHeaderCloseButtonSize.toPx() +
        FolderPaneHeaderCloseButtonGap.toPx()
    }
    val targetTextWidthPx = max(0f, resolvedPaneWidthPx - targetTextLeftPx - trailingReservedWidthPx)
    val sourceTextLeftPx = if (anchorContentRect == null) {
      targetTextLeftPx
    } else {
      anchorContentRect.left + horizontalInsetPx + sourceIconSizePx + sourceIconGapPx
    }
    val sourceTextWidthPx = if (anchorContentRect == null) {
      targetTextWidthPx
    } else {
      max(
        0f,
        anchorContentRect.width -
          (horizontalInsetPx + sourceIconSizePx + sourceIconGapPx) -
          trailingReservedWidthPx,
      )
    }
    val textLeftPx = lerpFloat(sourceTextLeftPx, targetTextLeftPx, progress)
    val textWidthPx = lerpFloat(sourceTextWidthPx, targetTextWidthPx, progress)
    val fontSizeSp = lerpFloat(14f, 17f, progress)

    Box(
      contentAlignment = Alignment.CenterStart,
      modifier = Modifier
        .offset { IntOffset(textLeftPx.roundToInt(), 0) }
        .width(with(density) { textWidthPx.toDp() })
        .height(FolderPaneHeaderTopHeight),
    ) {
      Text(
        text = title,
        style = AppTheme.typography.title.copy(fontSize = fontSizeSp.sp),
        maxLines = 1,
        overflow = TextOverflow.Ellipsis,
      )
    }
  }
}

@Composable
private fun FolderTopBarCloseButton(
  onClick: () -> Unit,
  modifier: Modifier = Modifier,
) {
  InteractionScope {
    Box(
      contentAlignment = Alignment.Center,
      modifier = modifier
        .size(FolderPaneHeaderCloseButtonSize)
        .clickable { onClick() }
        .pressScale(0.96f),
    ) {
      Box(
        contentAlignment = Alignment.Center,
        modifier = Modifier
          .size(FolderPaneHeaderCloseButtonVisualSize)
          .then(TopBarDefaults.controlShadowModifier(CircleShape))
          .background(TopBarDefaults.controlBackgroundColor(), CircleShape),
      ) {
        Icon(
          icon = Lucide.X,
          modifier = Modifier.size(FolderPaneHeaderCloseButtonIconSize),
          tint = AppTheme.colors.textPrimary,
        )
      }
    }
  }
}

@Composable
private fun PopoverScope.FolderTopBarActionList(
  onAction: (FolderAction, closePopover: () -> Unit) -> Unit,
) {
  Column(modifier = Modifier.fillMaxWidth()) {
    folderPrimaryActionSections().forEachIndexed { index, section ->
      if (index > 0) {
        FolderActionMenuDivider()
      }

      FolderTopBarActionSection(
        items = section.items,
        onAction = onAction,
      )
    }
  }
}

@Composable
private fun PopoverScope.FolderTopBarActionSection(
  items: List<FolderActionMenuItem>,
  onAction: (FolderAction, closePopover: () -> Unit) -> Unit,
) {
  PopoverList(
    items = items.map { action ->
      PopoverListItem(
        content = {
          FolderTopBarActionRow(
            action = action,
            modifier = Modifier
              .height(42.dp)
              .padding(horizontal = 16.dp),
          )
        },
        onSelected = {
          onAction(action.action) { close() }
        },
      )
    },
  )
}

@Composable
internal fun FolderActionMenuDivider() {
  CardDivider(
    inset = 16.dp,
    color = AppTheme.colors.borderDefault,
  )
}

@Composable
private fun FolderTopBarActionRow(
  action: FolderActionMenuItem,
  modifier: Modifier = Modifier,
) {
  val tint = if (action.isDanger) AppTheme.colors.danger else AppTheme.colors.textPrimary
  val trailingTint = if (action.isDanger) AppTheme.colors.danger else AppTheme.colors.textTertiary

  Row(
    modifier = modifier,
    verticalAlignment = Alignment.CenterVertically,
  ) {
    Icon(
      icon = action.icon,
      modifier = Modifier.size(18.dp),
      tint = tint,
    )

    Spacer(Modifier.width(12.dp))

    Text(
      text = action.label,
      modifier = Modifier.weight(1f),
      style = AppTheme.typography.action,
      color = tint,
    )

    if (action.trailingIcon != null) {
      Icon(
        icon = action.trailingIcon,
        modifier = Modifier.size(14.dp),
        tint = trailingTint,
      )
    }
  }
}

private fun lerpFloat(start: Float, end: Float, fraction: Float): Float {
  return start + (end - start) * fraction
}
