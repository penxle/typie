package co.typie.screen.space

import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.hapticfeedback.HapticFeedbackType
import androidx.compose.ui.platform.LocalHapticFeedback
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import co.typie.ext.safeBottomPadding
import co.typie.ext.verticalScroll
import co.typie.graphql.RefetchOnAppResumeEffect
import co.typie.graphql.RefetchOnScreenEnterEffect
import co.typie.graphql.RefetchOnSiteUpdateEffect
import co.typie.graphql.QueryState
import co.typie.icons.Lucide
import co.typie.navigation.Nav
import co.typie.route.Route
import co.typie.shell.LocalBottomBarState
import co.typie.ui.component.ErrorDialog
import co.typie.ui.component.Screen
import co.typie.ui.component.SpacePopover
import co.typie.ui.component.SpacePopoverLeadingKey
import co.typie.ui.component.Text
import co.typie.ui.component.formatSpaceSummary
import co.typie.ui.component.entity_container.EntityContainerEditAction
import co.typie.ui.component.entity_container.EntityContainerListContent
import co.typie.ui.component.entity_container.EntityContainerTopBarTrailing
import co.typie.ui.component.entity_container.EntityContainerTopBarTrailingKey
import co.typie.ui.component.entity_container.calculateEntityReorderOrdersFromOrderedKeys
import co.typie.ui.component.entity_container.displayOrderedEntityItems
import co.typie.ui.component.reorder.rememberReorderableListState
import co.typie.ui.component.reorder.reorderableListContainer
import co.typie.ui.component.topbar.ProvideTopBar
import co.typie.ui.component.topbar.topBarScrollOffset
import co.typie.ui.state.rememberScrollState
import co.typie.ui.theme.AppTheme
import kotlinx.coroutines.launch
import org.koin.compose.viewmodel.koinViewModel

@Composable
fun SpaceScreen() {
  val nav = Nav.current
  val haptic = LocalHapticFeedback.current
  val model = koinViewModel<SpaceViewModel>()
  val scrollState = rememberScrollState("space")
  val bottomBarState = LocalBottomBarState.current
  val presenterScope = rememberCoroutineScope()
  var isReordering by remember { mutableStateOf(false) }
  var isPersistingReorder by remember { mutableStateOf(false) }
  val siteId = model.siteId

  LaunchedEffect(Unit) {
    bottomBarState.visible = true
  }

  RefetchOnScreenEnterEffect(model::onScreenEntered)
  RefetchOnAppResumeEffect(model::refetch)
  RefetchOnSiteUpdateEffect(siteId = siteId, onRefetch = model::refetch)

  val site = (model.query.state as? QueryState.Success)?.data?.site
  LaunchedEffect(site?.id) {
    isReordering = false
    isPersistingReorder = false
  }

  val serverEntities = remember(site?.entities) {
    normalizeSpaceEntities(site?.entities.orEmpty())
  }
  val serverEntityIds = remember(serverEntities) { serverEntities.map { it.id } }
  val editActions = listOf(
    EntityContainerEditAction(
      icon = Lucide.SquareCheck,
      label = "여러 항목 선택하기",
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
    leadingKey = SpacePopoverLeadingKey,
    leading = { SpacePopover() },
    center = {
      Text(
        site?.name ?: "스페이스",
        style = AppTheme.typography.title,
        maxLines = 1,
        overflow = TextOverflow.Ellipsis,
      )
    },
    trailingKey = EntityContainerTopBarTrailingKey,
    trailing = if (serverEntities.isEmpty()) null else {
      {
        EntityContainerTopBarTrailing(
          isReordering = isReordering,
          actions = editActions,
          onDoneClick = { isReordering = false },
        )
      }
    },
    scrollOffset = scrollState.topBarScrollOffset(),
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
        keys = serverEntityIds,
        verticalScrollableState = scrollState,
      )
      val displayEntities = remember(serverEntities, reorderState.displayedKeys) {
        displayOrderedEntityItems(serverEntities, reorderState.displayedKeys)
      }

      EntityContainerListContent(
        items = displayEntities,
        emptyMessage = "문서와 폴더가 여기 나타나요",
        isReordering = isReordering,
        reorderState = reorderState,
        isPersistingReorder = isPersistingReorder,
        modifier = Modifier
          .fillMaxSize()
          .verticalScroll(scrollState)
          .padding(contentPadding)
          .safeBottomPadding()
          .reorderableListContainer(reorderState),
        header = {
          SpaceHeader(
            title = site?.name.orEmpty(),
            summary = formatSpaceSummary(
              folderCount = site?.folderCount ?: 0,
              documentCount = site?.documentCount ?: 0,
            ),
          )
        },
        onDocumentClick = { slug -> nav.navigate(Route.Editor(slug)) },
        onFolderClick = { entityId -> nav.navigate(Route.Folder(entityId)) },
        onDragStarted = {
          haptic.performHapticFeedback(HapticFeedbackType.GestureThresholdActivate)
        },
        onDragMoved = {
          haptic.performHapticFeedback(HapticFeedbackType.SegmentFrequentTick)
        },
        onDragStopped = onDragStopped@{ commit ->
          haptic.performHapticFeedback(HapticFeedbackType.GestureEnd)
          if (commit == null || commit.orderedKeys == serverEntityIds) {
            return@onDragStopped
          }

          val reorderOrders = calculateEntityReorderOrdersFromOrderedKeys(
            items = serverEntities,
            orderedKeys = commit.orderedKeys,
            movedKey = commit.movedKey,
          ) ?: run {
            reorderState.resetToServerKeys(serverEntityIds)
            return@onDragStopped
          }

          isPersistingReorder = true
          presenterScope.launch {
            val success = model.moveRootEntity(
              entityId = commit.movedKey,
              lowerOrder = reorderOrders.lowerOrder,
              upperOrder = reorderOrders.upperOrder,
            )
            isPersistingReorder = false

            if (!success) {
              reorderState.resetToServerKeys(serverEntityIds)
            }
          }
        },
      )
    },
  )
}

@Composable
private fun SpaceHeader(
  title: String,
  summary: String,
) {
  Column(
    modifier = Modifier
      .fillMaxWidth()
      .padding(horizontal = 16.dp)
      .padding(top = 4.dp, bottom = 24.dp),
  ) {
    Text(
      if (title.isBlank()) " " else title,
      style = AppTheme.typography.display,
      maxLines = 1,
      overflow = TextOverflow.Ellipsis,
    )

    Spacer(Modifier.height(8.dp))

    Text(
      summary,
      style = AppTheme.typography.body,
      color = AppTheme.colors.textTertiary,
    )
  }
}
