package co.typie.screen.folder

import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.FlowRow
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
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
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.mutableIntStateOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.hapticfeedback.HapticFeedbackType
import androidx.compose.ui.layout.onSizeChanged
import androidx.compose.ui.platform.LocalHapticFeedback
import androidx.compose.ui.platform.LocalDensity
import androidx.compose.ui.platform.LocalUriHandler
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.IntOffset
import androidx.compose.ui.unit.sp
import co.typie.ext.safeBottomPadding
import co.typie.ext.InteractionScope
import co.typie.ext.clickable
import co.typie.ext.pressScale
import co.typie.ext.verticalScroll
import co.typie.graphql.RefetchOnAppResumeEffect
import co.typie.graphql.QueryState
import co.typie.graphql.RefetchOnScreenEnterEffect
import co.typie.graphql.RefetchOnSiteUpdateEffect
import co.typie.icons.Lucide
import co.typie.navigation.Nav
import co.typie.overlay.Toast
import co.typie.overlay.ToastType
import co.typie.route.Route
import co.typie.screen.home.resolveEntityIconAppearance
import co.typie.shell.LocalBottomBarState
import co.typie.ui.component.ErrorDialog
import co.typie.ui.component.Screen
import co.typie.ui.component.Text
import co.typie.ui.component.formatFolderSummary
import co.typie.ui.component.ResponsiveContainerDefaults
import co.typie.ui.component.bottomsheet.LocalBottomSheetHost
import co.typie.ui.component.bottomsheet.showBottomSheetFromPopoverAction
import co.typie.ui.component.entity_container.EntityContainerEditAction
import co.typie.ui.component.entity_container.EntityContainerListContent
import co.typie.ui.component.entity_container.EntityContainerTopBarTrailing
import co.typie.ui.component.entity_container.EntityContainerTopBarTrailingKey
import co.typie.ui.component.popover.Popover
import co.typie.ui.component.popover.LocalPopoverPaneTransition
import co.typie.ui.component.popover.PopoverDefaults
import co.typie.ui.component.popover.PopoverList
import co.typie.ui.component.popover.PopoverListItem
import co.typie.ui.component.popover.PopoverPlacement
import co.typie.ui.component.popover.PopoverScope
import co.typie.ui.component.popover.PopoverTransitionElement
import co.typie.ui.component.popover.PopoverTransitionFrame
import co.typie.ui.component.reorder.rememberReorderableListState
import co.typie.ui.component.reorder.reorderableListContainer
import co.typie.ui.component.topbar.ProvideTopBar
import co.typie.ui.component.topbar.TopBarBackButton
import co.typie.ui.component.topbar.TopBarDefaults
import co.typie.ui.icon.Icon
import co.typie.ui.icon.IconData
import co.typie.ui.shape.SquircleShape
import co.typie.ui.state.rememberScrollState
import co.typie.ui.theme.AppTheme
import kotlin.math.max
import kotlin.math.roundToInt
import kotlinx.coroutines.launch
import org.koin.compose.koinInject
import org.koin.compose.viewmodel.koinViewModel

private val FolderTopBarVerticalOffset = (TopBarDefaults.Height - TopBarDefaults.ButtonSize) / 2
private val FolderTopBarCollapsedRadius = PopoverDefaults.ExpandedRadius
private val FolderTopBarPopoverScreenPadding = PaddingValues(
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

private data class FolderTopBarAction(
  val icon: IconData,
  val label: String,
  val trailingIcon: IconData? = null,
  val tint: Color? = null,
  val onClick: (closePopover: () -> Unit) -> Unit = { closePopover -> closePopover() },
)

private data class FolderTopBarActionSpec(
  val icon: IconData,
  val label: String,
  val trailingIcon: IconData? = null,
  val isDanger: Boolean = false,
  val opensExternalUrl: Boolean = false,
  val opensRenameSheet: Boolean = false,
  val opensIconSheet: Boolean = false,
  val opensShareSheet: Boolean = false,
)

private data class FolderVisibilityPresentation(
  val label: String,
  val isShared: Boolean,
)

@Composable
fun FolderScreen(entityId: String) {
  val nav = Nav.current
  val haptic = LocalHapticFeedback.current
  val uriHandler = LocalUriHandler.current
  val bottomSheetHost = LocalBottomSheetHost.current
  val presenterScope = rememberCoroutineScope()
  val toast = koinInject<Toast>()
  val model = koinViewModel<FolderViewModel>(key = "folder:$entityId")
  val scrollState = rememberScrollState("folder-scroll:$entityId")
  val bottomBarState = LocalBottomBarState.current
  var isReordering by remember { mutableStateOf(false) }
  var isPersistingReorder by remember { mutableStateOf(false) }
  val siteId = model.siteId

  LaunchedEffect(Unit) {
    bottomBarState.visible = true
  }

  LaunchedEffect(entityId) {
    model.entityId = entityId
    isReordering = false
    isPersistingReorder = false
  }

  RefetchOnScreenEnterEffect(model::onScreenEntered)
  RefetchOnAppResumeEffect(model::refetch)
  RefetchOnSiteUpdateEffect(siteId = siteId, onRefetch = model::refetch)

  val entity = (model.query.state as? QueryState.Success)?.data?.entity
  val folder = entity?.node?.onFolder
  val folderTitle = folder?.name ?: "폴더"
  val folderSummary = formatFolderSummary(
    folderCount = folder?.folderCount ?: 0,
    documentCount = folder?.documentCount ?: 0,
  )
  val breadcrumbNames = buildList {
    add(entity?.site?.name ?: "내 스페이스")
    addAll(entity?.ancestors?.mapNotNull { it.node.onFolder?.name }.orEmpty())
  }
  val serverChildren = remember(entity?.children) {
    normalizeFolderChildren(entity?.children.orEmpty())
  }
  val serverChildIds = remember(serverChildren) { serverChildren.map { it.id } }
  val centerActions = folderTopBarCenterActions().map { action ->
    FolderTopBarAction(
      icon = action.icon,
      label = action.label,
      trailingIcon = action.trailingIcon,
      tint = if (action.isDanger) AppTheme.colors.danger else null,
      onClick = when {
        action.opensRenameSheet -> { closePopover ->
          val resolvedFolder = folder
          if (resolvedFolder != null) {
            showBottomSheetFromPopoverAction(
              closePopover = closePopover,
              presenterScope = presenterScope,
              bottomSheetHost = bottomSheetHost,
            ) {
              FolderRenameSheet(
                model = model,
                folderId = resolvedFolder.id,
                initialName = resolvedFolder.name,
              )
            }
          } else {
            closePopover()
          }
        }

        action.opensIconSheet -> { closePopover ->
          val resolvedEntity = entity
          val resolvedFolder = folder
          if (resolvedEntity != null && resolvedFolder != null) {
            showBottomSheetFromPopoverAction(
              closePopover = closePopover,
              presenterScope = presenterScope,
              bottomSheetHost = bottomSheetHost,
            ) {
              FolderIconPickerSheet(
                model = model,
                entityId = resolvedEntity.id,
                initialIcon = resolvedEntity.icon,
                initialColor = resolvedEntity.iconColor,
              )
            }
          } else {
            closePopover()
          }
        }

        action.opensShareSheet -> { closePopover ->
          val resolvedEntity = entity
          val resolvedFolder = folder
          if (resolvedEntity != null && resolvedFolder != null) {
            // TODO: Track folder share sheet open.
            showBottomSheetFromPopoverAction(
              closePopover = closePopover,
              presenterScope = presenterScope,
              bottomSheetHost = bottomSheetHost,
            ) {
              FolderShareSheet(
                model = model,
                folderId = resolvedFolder.id,
                folderUrl = resolvedEntity.url,
                initialVisibility = resolvedEntity.visibility,
                initialThumbnailUrl = resolvedFolder.thumbnail?.url,
              )
            }
          } else {
            closePopover()
          }
        }

        action.opensExternalUrl -> { closePopover ->
          closePopover()
          entity?.url?.let(uriHandler::openUri)
        }

        else -> { closePopover ->
          closePopover()
          toast.show(ToastType.Notification, "준비 중인 기능이에요.")
        }
      },
    )
  }
  val editActions = listOf(
    EntityContainerEditAction(
      icon = Lucide.SquareCheck,
      label = "여러 항목 선택하기",
      onClick = { closePopover ->
        closePopover()
        toast.show(ToastType.Notification, "준비 중인 기능이에요.")
      },
    ),
    EntityContainerEditAction(
      icon = Lucide.ChevronsUpDown,
      label = "순서 변경하기",
      onClick = { closePopover ->
        closePopover()
        isReordering = true
      },
    ),
  )

  ProvideTopBar(
    leading = { TopBarBackButton() },
    center = {
      Box(
        contentAlignment = Alignment.Center,
        modifier = Modifier.fillMaxWidth(),
      ) {
        if (isReordering) {
          FolderTopBarCapsule(
            title = folderTitle,
            subtitle = folderSummary,
            iconName = entity?.icon,
            iconColor = entity?.iconColor,
            modifier = Modifier
              .fillMaxWidth()
              .widthIn(max = ResponsiveContainerDefaults.MaxWidth),
          )
        } else {
          FolderTopBarCenterMenu(
            title = folderTitle,
            subtitle = folderSummary,
            breadcrumbNames = breadcrumbNames,
            visibilityName = entity?.visibility?.name,
            availabilityName = entity?.availability?.name,
            iconName = entity?.icon,
            iconColor = entity?.iconColor,
            actions = centerActions,
            modifier = Modifier
              .fillMaxWidth()
              .widthIn(max = ResponsiveContainerDefaults.MaxWidth),
          )
        }
      }
    },
    trailingKey = EntityContainerTopBarTrailingKey,
    trailing = if (serverChildren.isEmpty()) null else {
      {
        EntityContainerTopBarTrailing(
          isReordering = isReordering,
          actions = editActions,
          onDoneClick = { isReordering = false },
        )
      }
    },
  )

  if (model.query.state is QueryState.Error) {
    ErrorDialog { model.refetch() }
  }

  Screen(
    loading = model.query.state !is QueryState.Success,
    background = AppTheme.colors.surfaceBase,
    contentPadding = PaddingValues(0.dp),
    primaryScrollableState = scrollState,
    body = { contentPadding ->
      val reorderState = rememberReorderableListState(
        keys = serverChildIds,
        verticalScrollableState = scrollState,
      )
      val displayChildren = remember(serverChildren, reorderState.displayedKeys) {
        displayFolderChildren(serverChildren, reorderState.displayedKeys)
      }

      EntityContainerListContent(
        items = displayChildren,
        emptyMessage = "폴더가 비어 있어요",
        isReordering = isReordering,
        reorderState = reorderState,
        isPersistingReorder = isPersistingReorder,
        modifier = Modifier
          .fillMaxSize()
          .verticalScroll(scrollState)
          .padding(contentPadding)
          .safeBottomPadding()
          .reorderableListContainer(reorderState),
        onDocumentClick = { slug -> nav.navigate(Route.Editor(slug)) },
        onFolderClick = { childEntityId -> nav.navigate(Route.Folder(childEntityId)) },
        onDragStarted = {
          haptic.performHapticFeedback(HapticFeedbackType.GestureThresholdActivate)
        },
        onDragMoved = {
          haptic.performHapticFeedback(HapticFeedbackType.SegmentFrequentTick)
        },
        onDragStopped = onDragStopped@{ commit ->
          haptic.performHapticFeedback(HapticFeedbackType.GestureEnd)
          if (commit == null || commit.orderedKeys == serverChildIds) {
            return@onDragStopped
          }

          val parentEntityId = entity?.id ?: run {
            reorderState.resetToServerKeys(serverChildIds)
            return@onDragStopped
          }
          val reorderOrders = calculateFolderReorderOrdersFromOrderedKeys(
            items = serverChildren,
            orderedKeys = commit.orderedKeys,
            movedKey = commit.movedKey,
          ) ?: run {
            reorderState.resetToServerKeys(serverChildIds)
            return@onDragStopped
          }

          isPersistingReorder = true
          presenterScope.launch {
            val success = model.moveChildEntity(
              entityId = commit.movedKey,
              parentEntityId = parentEntityId,
              lowerOrder = reorderOrders.lowerOrder,
              upperOrder = reorderOrders.upperOrder,
            )
            isPersistingReorder = false

            if (!success) {
              reorderState.resetToServerKeys(serverChildIds)
            }
          }
        },
      )
    },
  )
}

@Composable
private fun FolderTopBarCenterMenu(
  title: String,
  subtitle: String,
  breadcrumbNames: List<String>,
  visibilityName: String?,
  availabilityName: String?,
  iconName: String?,
  iconColor: String?,
  actions: List<FolderTopBarAction>,
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
        actions = actions,
      )
    },
  )
}

@Composable
private fun PopoverScope.FolderTopBarCenterPane(
  title: String,
  subtitle: String,
  breadcrumbNames: List<String>,
  visibilityName: String?,
  availabilityName: String?,
  iconName: String?,
  iconColor: String?,
  actions: List<FolderTopBarAction>,
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

    FolderTopBarActionList(actions = actions)
  }
}

@Composable
private fun FolderTopBarCapsule(
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
  visibilityName: String?,
  availabilityName: String?,
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
    visibilityName = visibilityName,
    availabilityName = availabilityName,
  )

  Column(
    modifier = Modifier
      .fillMaxWidth(),
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
        onClick = { onClose() },
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
  onClick: suspend () -> Unit,
  modifier: Modifier = Modifier,
) {
  InteractionScope {
    Box(
      contentAlignment = Alignment.Center,
      modifier = modifier
        .size(FolderPaneHeaderCloseButtonSize)
        .clickable(onClick)
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
  actions: List<FolderTopBarAction>,
) {
  PopoverList(
    items = actions.map { action ->
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
          action.onClick { close() }
        },
      )
    },
  )
}

@Composable
private fun FolderTopBarActionRow(
  action: FolderTopBarAction,
  modifier: Modifier = Modifier,
) {
  Row(
    modifier = modifier,
    verticalAlignment = Alignment.CenterVertically,
  ) {
    Icon(
      icon = action.icon,
      modifier = Modifier.size(18.dp),
      tint = action.tint ?: AppTheme.colors.textPrimary,
    )

    Spacer(Modifier.width(12.dp))

    Text(
      text = action.label,
      modifier = Modifier.weight(1f),
      style = AppTheme.typography.action,
      color = action.tint ?: AppTheme.colors.textPrimary,
    )

    if (action.trailingIcon != null) {
      Icon(
        icon = action.trailingIcon,
        modifier = Modifier.size(14.dp),
        tint = action.tint ?: AppTheme.colors.textTertiary,
      )
    }
  }
}

private fun folderTopBarCenterActions(): List<FolderTopBarActionSpec> {
  return listOf(
    FolderTopBarActionSpec(
      icon = Lucide.PenLine,
      label = "이름 변경",
      opensRenameSheet = true,
    ),
    FolderTopBarActionSpec(
      icon = Lucide.Palette,
      label = "아이콘 변경",
      opensIconSheet = true,
    ),
    FolderTopBarActionSpec(
      icon = Lucide.Globe,
      label = "스페이스에서 열기",
      trailingIcon = Lucide.ExternalLink,
      opensExternalUrl = true,
    ),
    FolderTopBarActionSpec(
      icon = Lucide.Blend,
      label = "공유 및 게시",
      opensShareSheet = true,
    ),
    FolderTopBarActionSpec(
      icon = Lucide.ClipboardCopy,
      label = "복사",
    ),
    FolderTopBarActionSpec(
      icon = Lucide.Scissors,
      label = "잘라내기",
    ),
    FolderTopBarActionSpec(
      icon = Lucide.Trash2,
      label = "삭제",
      isDanger = true,
    ),
  )
}

private fun folderVisibilityPresentation(
  visibilityName: String?,
  availabilityName: String?,
): FolderVisibilityPresentation {
  return when {
    visibilityName == "PUBLIC" -> FolderVisibilityPresentation(label = "공개", isShared = true)
    visibilityName == "UNLISTED" && availabilityName == "UNLISTED" ->
      FolderVisibilityPresentation(label = "링크 조회/편집 가능", isShared = true)
    visibilityName == "UNLISTED" ->
      FolderVisibilityPresentation(label = "링크 조회 가능", isShared = true)
    availabilityName == "UNLISTED" ->
      FolderVisibilityPresentation(label = "링크 편집 가능", isShared = true)
    else -> FolderVisibilityPresentation(label = "비공개", isShared = false)
  }
}

private fun lerpFloat(start: Float, end: Float, fraction: Float): Float {
  return start + (end - start) * fraction
}
