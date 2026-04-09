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
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.LaunchedEffect
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
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.IntOffset
import androidx.compose.ui.unit.sp
import co.typie.ext.safeBottomPadding
import co.typie.ext.verticalScroll
import co.typie.graphql.FolderScreen_Query
import co.typie.graphql.QueryState
import co.typie.icons.Lucide
import co.typie.navigation.Nav
import co.typie.route.Route
import co.typie.screen.home.resolveEntityIconAppearance
import co.typie.shell.LocalBottomBarState
import co.typie.ui.component.ErrorDialog
import co.typie.ui.component.EntityListCard
import co.typie.ui.component.EntityListItem
import co.typie.ui.component.Screen
import co.typie.ui.component.Text
import co.typie.ui.component.formatFolderSummary
import co.typie.ui.component.popover.Popover
import co.typie.ui.component.popover.PopoverDefaults
import co.typie.ui.component.popover.PopoverList
import co.typie.ui.component.popover.PopoverListItem
import co.typie.ui.component.popover.PopoverPlacement
import co.typie.ui.component.popover.PopoverScope
import co.typie.ui.component.popover.LocalPopoverPaneTransition
import co.typie.ui.component.popover.PopoverTransitionElement
import co.typie.ui.component.popover.PopoverTransitionFrame
import co.typie.ui.component.ResponsiveContainerDefaults
import co.typie.ui.component.topbar.ProvideTopBar
import co.typie.ui.component.topbar.TopBarBackButton
import co.typie.ui.component.topbar.TopBarButton
import co.typie.ui.component.topbar.TopBarDefaults
import co.typie.ui.icon.Icon
import co.typie.ui.icon.IconData
import co.typie.ui.shape.SquircleShape
import co.typie.ui.state.rememberScrollState
import co.typie.ui.theme.AppTheme
import kotlin.math.max
import kotlin.math.roundToInt
import org.koin.compose.viewmodel.koinViewModel

private val FolderTopBarVerticalOffset = (TopBarDefaults.Height - TopBarDefaults.ButtonSize) / 2
private val FolderTopBarCollapsedRadius = PopoverDefaults.ExpandedRadius
private val FolderTopBarTrailingKey = Any()
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

private data class FolderTopBarAction(
  val icon: IconData,
  val label: String,
  val tint: Color? = null,
  val onClick: () -> Unit = {},
)

private data class FolderVisibilityPresentation(
  val label: String,
  val isShared: Boolean,
)

@Composable
fun FolderScreen(entityId: String) {
  val nav = Nav.current
  val model = koinViewModel<FolderViewModel>(key = "folder:$entityId")
  val scrollState = rememberScrollState("folder-scroll:$entityId")
  val bottomBarState = LocalBottomBarState.current

  LaunchedEffect(Unit) {
    bottomBarState.visible = true
  }

  LaunchedEffect(entityId) {
    model.entityId = entityId
  }

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
  val items = entity?.children?.mapNotNull { it.toListItem() }.orEmpty()
  val centerActions = listOf(
    FolderTopBarAction(icon = Lucide.FolderSymlink, label = "다른 폴더로 옮기기"),
    FolderTopBarAction(icon = Lucide.ExternalLink, label = "스페이스에서 열기"),
    FolderTopBarAction(icon = Lucide.Blend, label = "공유하기"),
    FolderTopBarAction(icon = Lucide.PenLine, label = "이름 바꾸기"),
    FolderTopBarAction(icon = Lucide.Trash2, label = "삭제하기", tint = AppTheme.colors.danger),
  )
  val editActions = listOf(
    FolderTopBarAction(icon = Lucide.SquareCheck, label = "여러 항목 선택하기"),
    FolderTopBarAction(icon = Lucide.ChevronsUpDown, label = "순서 변경하기"),
  )

  ProvideTopBar(
    leading = { TopBarBackButton() },
    center = {
      Box(
        contentAlignment = Alignment.Center,
        modifier = Modifier.fillMaxWidth(),
      ) {
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
    },
    trailingKey = FolderTopBarTrailingKey,
    trailing = if (items.isEmpty()) null else {
      {
        FolderTopBarEditMenu(actions = editActions)
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
      Column(
        modifier = Modifier
          .fillMaxSize()
          .verticalScroll(scrollState)
          .padding(contentPadding)
          .safeBottomPadding(),
      ) {
        EntityListCard(
          items = items,
          emptyMessage = "폴더가 비어 있어요",
          modifier = Modifier.padding(horizontal = 16.dp),
          onDocumentClick = { slug -> nav.navigate(Route.Editor(slug)) },
          onFolderClick = { childEntityId -> nav.navigate(Route.Folder(childEntityId)) },
        )

        Spacer(Modifier.height(140.dp))
      }
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
  FlowRow(
    horizontalArrangement = Arrangement.spacedBy(4.dp),
    verticalArrangement = Arrangement.spacedBy(2.dp),
  ) {
    names.forEachIndexed { index, name ->
      if (index == 0) {
        Text(
          text = name,
          style = AppTheme.typography.caption.copy(fontSize = 14.sp),
          color = AppTheme.colors.textTertiary,
        )
      } else {
        Row(
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
            style = AppTheme.typography.caption.copy(fontSize = 14.sp),
            color = AppTheme.colors.textTertiary,
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
    val targetTextWidthPx = max(0f, resolvedPaneWidthPx - targetTextLeftPx - with(density) { 16.dp.toPx() })
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
        anchorContentRect.width - (horizontalInsetPx + sourceIconSizePx + sourceIconGapPx) - horizontalInsetPx,
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
private fun FolderTopBarEditMenu(
  actions: List<FolderTopBarAction>,
) {
  Popover(
    placement = PopoverPlacement.BelowEnd,
    anchor = { TopBarButton(icon = Lucide.LayoutList) },
    pane = {
      Column(modifier = Modifier.padding(PopoverDefaults.PanePadding)) {
        FolderTopBarActionList(actions = actions)
      }
    },
  )
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
          close()
          action.onClick()
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
      style = AppTheme.typography.action,
      color = action.tint ?: AppTheme.colors.textPrimary,
    )
  }
}

private fun FolderScreen_Query.Child.toListItem(): EntityListItem? {
  val childFolder = node.onFolder
  if (childFolder != null) {
    return EntityListItem.Folder(
      id = id,
      iconName = icon,
      iconColor = iconColor,
      name = childFolder.name,
      folderCount = childFolder.folderCount,
      documentCount = childFolder.documentCount,
    )
  }

  val document = node.onDocument
  if (document != null) {
    return EntityListItem.Document(
      id = id,
      iconName = icon,
      iconColor = iconColor,
      slug = slug,
      title = document.title,
      subtitle = document.subtitle,
      excerpt = document.excerpt,
      updatedAt = document.updatedAt,
    )
  }

  return null
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
