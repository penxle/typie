package co.typie.screen.folder

import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.widthIn
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.hapticfeedback.HapticFeedbackType
import androidx.compose.ui.platform.LocalHapticFeedback
import androidx.compose.ui.platform.LocalUriHandler
import co.typie.ext.safeBottomPadding
import co.typie.ext.verticalScroll
import co.typie.graphql.QueryState
import co.typie.graphql.RefetchOnAppResumeEffect
import co.typie.graphql.RefetchOnScreenEnterEffect
import co.typie.graphql.RefetchOnSiteUpdateEffect
import co.typie.icons.Lucide
import co.typie.navigation.Nav
import co.typie.overlay.Toast
import co.typie.overlay.ToastType
import co.typie.route.Route
import co.typie.screen.entity_move.EntityMoveSheet
import co.typie.screen.entity_move.MoveSourceEntity
import co.typie.shell.LocalBottomBarState
import co.typie.ui.component.ErrorDialog
import co.typie.ui.component.ResponsiveContainerDefaults
import co.typie.ui.component.Screen
import co.typie.ui.component.bottomsheet.LocalBottomSheetHost
import co.typie.ui.component.bottomsheet.showBottomSheetFromPopoverAction
import co.typie.ui.component.entity_container.EntityContainerEditAction
import co.typie.ui.component.entity_container.EntityContainerListContent
import co.typie.ui.component.entity_container.EntityContainerTopBarTrailing
import co.typie.ui.component.entity_container.EntityContainerTopBarTrailingKey
import co.typie.ui.component.entity_container.calculateEntityReorderOrdersFromOrderedKeys
import co.typie.ui.component.entity_container.displayOrderedEntityItems
import co.typie.ui.component.reorder.rememberReorderableListState
import co.typie.ui.component.reorder.reorderableListContainer
import co.typie.ui.component.topbar.ProvideTopBar
import co.typie.ui.component.topbar.TopBarBackButton
import co.typie.ui.state.rememberScrollState
import co.typie.ui.theme.AppTheme
import org.koin.compose.koinInject
import org.koin.compose.viewmodel.koinViewModel

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
  val folderSummary = co.typie.ui.component.formatFolderSummary(
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
  val reorderState = rememberReorderableListState(
    keys = serverChildIds,
    verticalScrollableState = scrollState,
  )
  val displayChildren = remember(serverChildren, reorderState.displayedKeys) {
    displayOrderedEntityItems(serverChildren, reorderState.displayedKeys)
  }
  val centerActions = remember { folderTopBarCenterActions() }

  fun showUnavailable(closePopover: () -> Unit) {
    closePopover()
    toast.show(ToastType.Notification, "준비 중인 기능이에요.")
  }

  fun onCenterAction(action: FolderAction, closePopover: () -> Unit) {
    when (action) {
      FolderAction.Rename -> {
        val resolvedFolder = folder ?: return closePopover()
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
      }

      FolderAction.ChangeIcon -> {
        val resolvedEntity = entity
        val resolvedFolder = folder
        if (resolvedEntity == null || resolvedFolder == null) {
          closePopover()
          return
        }
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
      }

      FolderAction.Share -> {
        val resolvedEntity = entity
        val resolvedFolder = folder
        if (resolvedEntity == null || resolvedFolder == null) {
          closePopover()
          return
        }
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
      }

      FolderAction.Move -> {
        val resolvedEntity = entity
        val resolvedFolder = folder
        if (resolvedEntity == null || resolvedFolder == null) {
          closePopover()
          return
        }
        showBottomSheetFromPopoverAction(
          closePopover = closePopover,
          presenterScope = presenterScope,
          bottomSheetHost = bottomSheetHost,
        ) {
          EntityMoveSheet(
            source = MoveSourceEntity.Folder(
              id = resolvedEntity.id,
              title = resolvedFolder.name,
              depth = resolvedEntity.depth,
              maxDescendantFoldersDepth = resolvedFolder.maxDescendantFoldersDepth,
            ),
            onMoved = model::refetch,
          )
        }
      }

      FolderAction.OpenExternal -> {
        closePopover()
        entity?.url?.let(uriHandler::openUri)
      }

      FolderAction.StartReorder -> {
        closePopover()
        isReordering = true
      }

      FolderAction.SelectMultiple,
      FolderAction.Copy,
      FolderAction.Cut,
      FolderAction.Delete -> showUnavailable(closePopover)
    }
  }

  val editActions = listOf(
    EntityContainerEditAction(
      icon = Lucide.SquareCheck,
      label = "여러 항목 선택하기",
      onClick = { closePopover ->
        onCenterAction(FolderAction.SelectMultiple, closePopover)
      },
    ),
    EntityContainerEditAction(
      icon = Lucide.ChevronsUpDown,
      label = "순서 변경하기",
      onClick = { closePopover ->
        onCenterAction(FolderAction.StartReorder, closePopover)
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
            onAction = ::onCenterAction,
            modifier = Modifier
              .fillMaxWidth()
              .widthIn(max = ResponsiveContainerDefaults.MaxWidth),
          )
        }
      }
    },
    trailingKey = EntityContainerTopBarTrailingKey,
    trailing = if (displayChildren.isEmpty()) null else {
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
    contentPadding = PaddingValues(),
    primaryScrollableState = scrollState,
    body = { contentPadding ->
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
          val reorderOrders = calculateEntityReorderOrdersFromOrderedKeys(
            items = serverChildren,
            orderedKeys = commit.orderedKeys,
            movedKey = commit.movedKey,
          ) ?: run {
            reorderState.resetToServerKeys(serverChildIds)
            return@onDragStopped
          }

          isPersistingReorder = true
          model.moveChildEntity(
            entityId = commit.movedKey,
            parentEntityId = parentEntityId,
            lowerOrder = reorderOrders.lowerOrder,
            upperOrder = reorderOrders.upperOrder,
          ) { success ->
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
