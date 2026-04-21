package co.typie.screen.space.folder

import androidx.compose.foundation.background
import androidx.compose.foundation.border
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
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableIntStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.layout.onSizeChanged
import androidx.compose.ui.platform.LocalDensity
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.IntOffset
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import co.typie.domain.entity.EntityAction
import co.typie.domain.entity.EntityBreadcrumbLayout
import co.typie.domain.entity.EntityHeader
import co.typie.domain.entity.EntityHeaderDefaults
import co.typie.domain.entity.EntityIcon
import co.typie.domain.entity.entityItemActionSections
import co.typie.domain.entity.entityVisibilityPresentation
import co.typie.ext.InteractionScope
import co.typie.ext.clickable
import co.typie.ext.pressScale
import co.typie.graphql.fragment.FolderTopBar_entity
import co.typie.icons.Lucide
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
import co.typie.ui.component.popover.close
import co.typie.ui.component.topbar.TopBarDefaults
import co.typie.ui.icon.Icon
import co.typie.ui.skeleton.Skeleton
import co.typie.ui.theme.AppShapes
import co.typie.ui.theme.AppTheme
import co.typie.ui.theme.shadow
import kotlin.math.max
import kotlin.math.roundToInt

private val FolderTopBarVerticalOffset = (TopBarDefaults.Height - TopBarDefaults.ButtonSize) / 2
private val FolderTopBarCollapsedRadius = PopoverDefaults.ExpandedRadius
internal val FolderTopBarPopoverScreenPadding =
  PaddingValues(
    start = PopoverDefaults.ScreenPadding,
    top = FolderTopBarVerticalOffset,
    end = PopoverDefaults.ScreenPadding,
    bottom = FolderTopBarVerticalOffset + 100.dp,
  )
private val FolderPaneHeaderTopHeight = 28.dp
private val FolderPaneHeaderSourceHorizontalInset = 14.dp
private val FolderPaneHeaderSourceIconSize = 18.dp
private val FolderPaneHeaderSourceIconGap = 10.dp
private val FolderPaneHeaderCloseButtonSize = 44.dp

@Composable
internal fun FolderTopBarCenterMenu(
  title: String,
  subtitle: String,
  entity: FolderTopBar_entity,
  loading: Boolean,
  onAction: (EntityAction) -> Unit,
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
      Skeleton.Passive(enabled = loading) {
        FolderTopBarCapsule(
          title = title,
          subtitle = subtitle,
          entity = entity,
          modifier = modifier.pressScale(1.05f),
        )
      }
    },
    pane = {
      FolderTopBarCenterPane(
        title = title,
        subtitle = subtitle,
        entity = entity,
        loading = loading,
        onAction = onAction,
      )
    },
  )
}

@Composable
context(_: PopoverScope)
internal fun FolderTopBarCenterPane(
  title: String,
  subtitle: String,
  entity: FolderTopBar_entity,
  loading: Boolean,
  onAction: (EntityAction) -> Unit,
) {
  Column(
    modifier =
      Modifier.padding(
          start = PopoverDefaults.PanePadding,
          top = PopoverDefaults.PanePadding + 4.dp,
          end = PopoverDefaults.PanePadding,
          bottom = PopoverDefaults.PanePadding,
        )
        .fillMaxWidth()
        .widthIn(min = 280.dp, max = ResponsiveContainerDefaults.MaxWidth)
  ) {
    FolderTopBarPaneHeader(
      title = title,
      subtitle = subtitle,
      entity = entity,
      loading = loading,
      onClose = { close() },
    )

    Spacer(Modifier.height(6.dp))

    Column(modifier = Modifier.fillMaxWidth()) {
      entityItemActionSections().forEachIndexed { index, section ->
        if (index > 0) {
          CardDivider(inset = 16.dp, color = AppTheme.colors.borderDefault)
        }

        PopoverList(
          items =
            section.items.map { action ->
              PopoverListItem(
                enabled = !loading,
                content = {
                  Row(
                    modifier = Modifier.height(42.dp).padding(horizontal = 16.dp),
                    verticalAlignment = Alignment.CenterVertically,
                  ) {
                    Icon(
                      icon = action.icon,
                      modifier = Modifier.size(18.dp),
                      tint =
                        if (action.isDanger) AppTheme.colors.danger else AppTheme.colors.textDefault,
                    )

                    Spacer(Modifier.width(12.dp))

                    Text(
                      text = action.label,
                      modifier = Modifier.weight(1f),
                      style = AppTheme.typography.action,
                      color =
                        if (action.isDanger) AppTheme.colors.danger else AppTheme.colors.textDefault,
                    )

                    if (action.trailingIcon != null) {
                      Icon(
                        icon = action.trailingIcon,
                        modifier = Modifier.size(14.dp),
                        tint =
                          if (action.isDanger) AppTheme.colors.danger else AppTheme.colors.textMuted,
                      )
                    }
                  }
                },
                onSelected = {
                  close()
                  onAction(action.action)
                },
              )
            }
        )
      }
    }
  }
}

@Composable
internal fun FolderTopBarCapsule(
  title: String,
  subtitle: String,
  entity: FolderTopBar_entity,
  modifier: Modifier = Modifier,
) {
  val shape = AppShapes.squircle(FolderTopBarCollapsedRadius)

  Row(
    verticalAlignment = Alignment.CenterVertically,
    modifier =
      modifier
        .height(TopBarDefaults.TitleHeight)
        .shadow(AppTheme.shadows.sm, shape)
        .border(1.dp, TopBarDefaults.controlBorderColor(), shape)
        .background(TopBarDefaults.controlBackgroundColor(), shape)
        .padding(horizontal = 14.dp),
  ) {
    EntityIcon(
      entity = entity.entityIcon_entity,
      modifier = Modifier.size(TopBarDefaults.TitleIconSize),
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
        color = AppTheme.colors.textMuted,
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
  entity: FolderTopBar_entity,
  loading: Boolean,
  onClose: () -> Unit,
) {
  val visibility = entityVisibilityPresentation(entity.visibility, entity.availability)
  val breadcrumbEntity = entity.entityBreadcrumb_entity
  val breadcrumbColor = AppTheme.colors.textMuted
  val visibilityColor =
    if (visibility.isShared) AppTheme.colors.textDefault else AppTheme.colors.textHint
  val subtitleColor = AppTheme.colors.textHint

  EntityHeader(
    topContentModifier = Modifier.fillMaxWidth().height(FolderPaneHeaderTopHeight),
    supportingContentPadding =
      PaddingValues(
        start =
          EntityHeaderDefaults.SupportingContentEndInset +
            EntityHeaderDefaults.IconSize +
            EntityHeaderDefaults.TitleGap,
        end = EntityHeaderDefaults.SupportingContentEndInset,
      ),
    topContent = {
      Box(modifier = Modifier.fillMaxWidth().height(FolderPaneHeaderTopHeight)) {
        PopoverTransitionElement(
          collapsedFrame =
            PopoverTransitionFrame(
              left = FolderPaneHeaderSourceHorizontalInset,
              top = (TopBarDefaults.ButtonSize - FolderPaneHeaderSourceIconSize) / 2,
              width = FolderPaneHeaderSourceIconSize,
              height = FolderPaneHeaderSourceIconSize,
            ),
          expandedFrame =
            PopoverTransitionFrame(
              left = EntityHeaderDefaults.SupportingContentEndInset,
              top = (FolderPaneHeaderTopHeight - EntityHeaderDefaults.IconSize) / 2,
              width = EntityHeaderDefaults.IconSize,
              height = EntityHeaderDefaults.IconSize,
            ),
        ) {
          Skeleton.Passive(enabled = loading) {
            EntityIcon(
              entity = entity.entityIcon_entity,
              modifier = Modifier.size(EntityHeaderDefaults.IconSize),
            )
          }
        }

        FolderTopBarTransitionTitle(title = title, loading = loading)

        FolderTopBarCloseButton(
          onClick = onClose,
          modifier = Modifier.align(Alignment.CenterEnd).padding(end = 6.dp),
        )
      }
    },
  ) {
    breadcrumb(
      entity = breadcrumbEntity,
      layout = EntityBreadcrumbLayout.SingleLineEllipsis,
      color = breadcrumbColor,
      loading = loading,
    )
    supportingText(text = visibility.label, color = visibilityColor, loading = loading)
    supportingText(
      text = subtitle,
      color = subtitleColor,
      loading = loading,
      placeholderLength = 10..18,
    )
  }
}

@Composable
private fun FolderTopBarTransitionTitle(title: String, loading: Boolean) {
  val density = LocalDensity.current
  val transition = LocalPopoverPaneTransition.current
  val progress = (transition?.progress ?: 1f).coerceIn(0f, 1f)
  val anchorContentRect = transition?.anchorContentRect
  var paneWidthPx by remember { mutableIntStateOf(0) }

  Box(
    modifier =
      Modifier.fillMaxWidth()
        .onSizeChanged { paneWidthPx = it.width }
        .height(FolderPaneHeaderTopHeight)
  ) {
    val resolvedPaneWidthPx =
      if (paneWidthPx > 0) {
        paneWidthPx.toFloat()
      } else {
        max(anchorContentRect?.width ?: 0f, with(density) { 280.dp.toPx() })
      }
    val horizontalInsetPx = with(density) { FolderPaneHeaderSourceHorizontalInset.toPx() }
    val sourceIconSizePx = with(density) { FolderPaneHeaderSourceIconSize.toPx() }
    val sourceIconGapPx = with(density) { FolderPaneHeaderSourceIconGap.toPx() }
    val targetTextLeftPx =
      with(density) {
        (EntityHeaderDefaults.SupportingContentEndInset +
            EntityHeaderDefaults.IconSize +
            EntityHeaderDefaults.TitleGap)
          .toPx()
      }
    val trailingReservedWidthPx =
      with(density) { 6.dp.toPx() + FolderPaneHeaderCloseButtonSize.toPx() + 8.dp.toPx() }
    val targetTextWidthPx =
      max(0f, resolvedPaneWidthPx - targetTextLeftPx - trailingReservedWidthPx)
    val sourceTextLeftPx =
      if (anchorContentRect == null) {
        targetTextLeftPx
      } else {
        anchorContentRect.left + horizontalInsetPx + sourceIconSizePx + sourceIconGapPx
      }
    val sourceTextWidthPx =
      if (anchorContentRect == null) {
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
      modifier =
        Modifier.offset { IntOffset(textLeftPx.roundToInt(), 0) }
          .width(with(density) { textWidthPx.toDp() })
          .height(FolderPaneHeaderTopHeight),
    ) {
      Skeleton.Passive(enabled = loading) {
        Text(
          text = title,
          style = AppTheme.typography.title.copy(fontSize = fontSizeSp.sp),
          maxLines = 1,
          overflow = TextOverflow.Ellipsis,
        )
      }
    }
  }
}

@Composable
private fun FolderTopBarCloseButton(onClick: () -> Unit, modifier: Modifier = Modifier) {
  InteractionScope {
    Box(
      contentAlignment = Alignment.Center,
      modifier =
        modifier.size(FolderPaneHeaderCloseButtonSize).clickable { onClick() }.pressScale(0.96f),
    ) {
      Box(
        contentAlignment = Alignment.Center,
        modifier =
          Modifier.size(24.dp)
            .shadow(AppTheme.shadows.md, AppShapes.circle)
            .clip(AppShapes.circle)
            .background(TopBarDefaults.controlBackgroundColor(), AppShapes.circle),
      ) {
        Icon(icon = Lucide.X, modifier = Modifier.size(16.dp), tint = AppTheme.colors.textDefault)
      }
    }
  }
}

private fun lerpFloat(start: Float, end: Float, fraction: Float): Float {
  return start + (end - start) * fraction
}
