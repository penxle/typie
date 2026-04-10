package co.typie.screen.space.folder

import androidx.compose.animation.AnimatedVisibility
import androidx.compose.animation.core.tween
import androidx.compose.animation.fadeIn
import androidx.compose.animation.fadeOut
import androidx.compose.animation.slideInVertically
import androidx.compose.animation.slideOutVertically
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.BoxWithConstraints
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.WindowInsets
import androidx.compose.foundation.layout.asPaddingValues
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.widthIn
import androidx.compose.runtime.Composable
import androidx.compose.runtime.DisposableEffect
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
import androidx.compose.ui.unit.dp
import co.typie.entity_transfer.EntityClipboardMode
import co.typie.entity_transfer.EntityClipboardService
import co.typie.entity_transfer.EntityPasteBar
import co.typie.entity_transfer.EntityPasteTarget
import co.typie.entity_transfer.EntityTransferSource
import co.typie.entity_transfer.entityPasteBarToastBottomInset
import co.typie.entity_transfer.toMessage
import co.typie.result.onErr
import co.typie.ext.safeBottomPadding
import co.typie.ext.safeDrawing
import co.typie.ext.verticalScroll
import co.typie.graphql.QueryState
import co.typie.graphql.RefetchOnAppResumeEffect
import co.typie.graphql.RefetchOnScreenEnterEffect
import co.typie.graphql.RefetchOnSiteUpdateEffect
import co.typie.icons.Lucide
import co.typie.navigation.LocalRoute
import co.typie.navigation.Nav
import co.typie.overlay.LocalToast
import co.typie.overlay.ToastType
import co.typie.result.onException
import co.typie.result.onOk
import co.typie.result.withDefaultExceptionHandler
import co.typie.route.Route
import co.typie.route.toastBottomInset
import co.typie.overlay.EntityMoveSheet
import co.typie.shell.SpaceBottomBarActionButton
import co.typie.ui.component.ConfirmModal
import co.typie.ui.component.ErrorDialog
import co.typie.ui.component.ResponsiveContainerDefaults
import co.typie.ui.component.Screen
import co.typie.shell.MainBottomBarPill
import co.typie.ui.component.bottombar.ProvideBottomBar
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
import co.typie.ui.component.sheet.LocalSheetHost
import co.typie.ui.component.topbar.ProvideTopBar
import co.typie.ui.component.topbar.TopBarBackButton
import co.typie.ui.component.topbar.TopBarDefaults
import co.typie.ui.state.rememberScrollState
import co.typie.ui.theme.AppTheme
import kotlinx.coroutines.launch
import org.koin.compose.koinInject
import org.koin.compose.viewmodel.koinViewModel

private val FolderScreenPasteBarBottomSpacerHeight = 176.dp
private val FolderScreenPasteBarBottomOffset = 28.dp

@Composable
fun FolderScreen(entityId: String) {
  val nav = Nav.current
  val haptic = LocalHapticFeedback.current
  val uriHandler = LocalUriHandler.current
  val bottomSheetHost = LocalBottomSheetHost.current
  val sheetHost = LocalSheetHost.current
  val presenterScope = rememberCoroutineScope()
  val toast = LocalToast.current
  val clipboard = koinInject<EntityClipboardService>()
  val model = koinViewModel<FolderViewModel>(key = "folder:$entityId")
  val scrollState = rememberScrollState("folder-scroll:$entityId")
  var isReordering by remember { mutableStateOf(false) }
  var isPersistingReorder by remember { mutableStateOf(false) }
  var isPasting by remember { mutableStateOf(false) }
  var animatedPasteBarVisible by remember { mutableStateOf(false) }
  var deleteRequest by remember(entityId) { mutableStateOf<FolderDeleteRequest?>(null) }
  val siteId = model.siteId

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
  val folderMetadataSummary = co.typie.ui.component.formatFolderMetadataSummary(
    folderCount = folder?.folderCount ?: 0,
    documentCount = folder?.documentCount ?: 0,
    characterCount = folder?.characterCount ?: 0,
  )
  val breadcrumbNames = buildList {
    add(entity?.site?.name ?: "내 스페이스")
    addAll(entity?.ancestors?.mapNotNull { it.node.onFolder?.name }.orEmpty())
  }
  val serverChildren = remember(entity?.children) {
    normalizeFolderChildren(
      siteName = entity?.site?.name.orEmpty(),
      children = entity?.children.orEmpty(),
    )
  }
  val serverChildIds = remember(serverChildren) { serverChildren.map { it.id } }
  val reorderState = rememberReorderableListState(
    keys = serverChildIds,
    verticalScrollableState = scrollState,
  )
  val displayChildren = remember(serverChildren, reorderState.displayedKeys) {
    displayOrderedEntityItems(serverChildren, reorderState.displayedKeys)
  }
  val clipboardState = clipboard.state
  val cutDimmedItemIds = remember(clipboardState) {
    if (clipboardState?.mode == EntityClipboardMode.Cut) {
      clipboardState.items.mapTo(mutableSetOf()) { it.id }
    } else {
      emptySet()
    }
  }
  val pasteTarget = remember(entity) {
    entity?.let { currentEntity ->
      EntityPasteTarget(
        siteId = currentEntity.site.id,
        destinationEntityId = currentEntity.id,
        destinationDepth = currentEntity.depth,
        ancestorFolderIds = currentEntity.ancestors.mapTo(mutableSetOf()) { it.id },
        lowerOrder = currentEntity.children.lastOrNull()?.order,
        upperOrder = null,
      )
    }
  }
  val isPasteBarVisible = clipboardState != null &&
    pasteTarget != null &&
    clipboard.canPaste(requireNotNull(pasteTarget))
  val isCurrentRoute = nav.current == LocalRoute.current
  val shouldShowPasteBar = isPasteBarVisible && isCurrentRoute
  val reservedBottomSpacerHeight = if (animatedPasteBarVisible) {
    FolderScreenPasteBarBottomSpacerHeight
  } else {
    140.dp
  }

  LaunchedEffect(shouldShowPasteBar) {
    animatedPasteBarVisible = shouldShowPasteBar
  }

  LaunchedEffect(isCurrentRoute, animatedPasteBarVisible) {
    if (!isCurrentRoute) {
      return@LaunchedEffect
    }
    toast.bottomInset = if (animatedPasteBarVisible) {
      entityPasteBarToastBottomInset(Route.Folder(entityId).toastBottomInset)
    } else {
      Route.Folder(entityId).toastBottomInset
    }
  }

  DisposableEffect(entityId) {
    onDispose {
      toast.bottomInset = Route.Folder(entityId).toastBottomInset
    }
  }

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
        closePopover()
        sheetHost.show(
          folderIconPickerSheet(
            model = model,
            entityId = resolvedEntity.id,
            initialIcon = resolvedEntity.icon,
            initialColor = resolvedEntity.iconColor,
          ),
        )
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
            source = EntityTransferSource.Folder(
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

      FolderAction.SelectMultiple -> showUnavailable(closePopover)

      FolderAction.Copy -> {
        val resolvedEntity = entity
        val resolvedFolder = folder
        if (resolvedEntity == null || resolvedFolder == null) {
          closePopover()
          return
        }
        clipboard.setCopy(
          sourceSiteId = resolvedEntity.site.id,
          items = listOf(
            EntityTransferSource.Folder(
              id = resolvedEntity.id,
              title = resolvedFolder.name,
              depth = resolvedEntity.depth,
              maxDescendantFoldersDepth = resolvedFolder.maxDescendantFoldersDepth,
            ),
          ),
        )
        closePopover()
      }

      FolderAction.Cut -> {
        val resolvedEntity = entity
        val resolvedFolder = folder
        if (resolvedEntity == null || resolvedFolder == null) {
          closePopover()
          return
        }
        clipboard.setCut(
          sourceSiteId = resolvedEntity.site.id,
          items = listOf(
            EntityTransferSource.Folder(
              id = resolvedEntity.id,
              title = resolvedFolder.name,
              depth = resolvedEntity.depth,
              maxDescendantFoldersDepth = resolvedFolder.maxDescendantFoldersDepth,
            ),
          ),
        )
        closePopover()
      }

      FolderAction.Delete -> {
        val resolvedFolder = folder
        if (resolvedFolder == null) {
          closePopover()
          return
        }
        closePopover()
        deleteRequest = FolderDeleteRequest(
          entityId = entityId,
          folderName = resolvedFolder.name,
          shouldPopOnSuccess = true,
        )
      }
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
            subtitle = folderMetadataSummary,
            breadcrumbNames = breadcrumbNames,
            visibilityName = entity?.visibility,
            availabilityName = entity?.availability,
            iconName = entity?.icon,
            iconColor = entity?.iconColor,
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

  ProvideBottomBar(
    pill = { MainBottomBarPill() },
    action = { SpaceBottomBarActionButton() },
  )

  if (model.query.state is QueryState.Error) {
    ErrorDialog { model.refetch() }
  }

  deleteRequest?.let { request ->
    ConfirmModal(
      title = "폴더 삭제",
      message = "\"${request.folderName}\" 폴더를 삭제하시겠어요? 삭제 후 30일 동안 휴지통에 보관돼요.",
      confirmText = "삭제하기",
      confirmIsDestructive = true,
      onConfirm = {
        deleteRequest = null
        presenterScope.launch {
          model.deleteFolderEntity(request.entityId)
            .withDefaultExceptionHandler(toast)
            .onOk {
              if (request.shouldPopOnSuccess) {
                nav.pop()
              } else {
                model.refetch()
              }
            }
        }
      },
      onDismiss = { deleteRequest = null },
    )
  }

  Screen(
    loading = model.query.state !is QueryState.Success,
    background = AppTheme.colors.surfaceBase,
    contentPadding = PaddingValues(),
    primaryScrollableState = scrollState,
    body = { contentPadding ->
      val reorderViewportTopInset = maxOf(
        0.dp,
        contentPadding.calculateTopPadding() - TopBarDefaults.BlurFadeHeight - TopBarDefaults.ContentTopSpacing,
      )
      val reorderViewportBottomInset =
        WindowInsets.safeDrawing.asPaddingValues().calculateBottomPadding() + 72.dp

      Box(
        modifier = Modifier
          .fillMaxSize()
          .reorderableListContainer(
            state = reorderState,
            viewportTopInset = reorderViewportTopInset,
            viewportBottomInset = reorderViewportBottomInset,
          ),
      ) {
        EntityContainerListContent(
          items = displayChildren,
          emptyMessage = "폴더가 비어 있어요",
          isReordering = isReordering,
          reorderState = reorderState,
          isPersistingReorder = isPersistingReorder,
          dimmedItemIds = cutDimmedItemIds,
          bottomSpacerHeight = reservedBottomSpacerHeight,
          modifier = Modifier
            .fillMaxSize()
            .verticalScroll(scrollState)
            .padding(contentPadding)
            .safeBottomPadding(),
          onDocumentClick = { slug -> nav.navigate(Route.Editor(slug)) },
          onFolderClick = { childEntityId -> nav.navigate(Route.Folder(childEntityId)) },
          onFolderLongPress = { item ->
            bottomSheetHost.show {
              FolderItemActionsSheet(
                item = item,
                actionScope = presenterScope,
                onAction = { action ->
                  when (action) {
                    FolderAction.Rename -> {
                      bottomSheetHost.show {
                        FolderRenameSheet(
                          model = model,
                          folderId = item.folderId,
                          initialName = item.name,
                        )
                      }
                    }

                    FolderAction.ChangeIcon -> {
                      sheetHost.await(
                        folderIconPickerSheet(
                          model = model,
                          entityId = item.id,
                          initialIcon = item.iconName,
                          initialColor = item.iconColor,
                        ),
                      )
                    }

                    FolderAction.OpenExternal -> uriHandler.openUri(item.url)

                    FolderAction.Share -> {
                      bottomSheetHost.show {
                        FolderShareSheet(
                          model = model,
                          folderId = item.folderId,
                          folderUrl = item.url,
                          initialVisibility = requireNotNull(item.visibility),
                          initialThumbnailUrl = item.thumbnailUrl,
                        )
                      }
                    }

                    FolderAction.Move -> {
                      bottomSheetHost.show {
                        EntityMoveSheet(
                          source = item.toTransferSource(),
                          onMoved = model::refetch,
                        )
                      }
                    }

                    FolderAction.Copy -> {
                      clipboard.setCopy(
                        sourceSiteId = siteId,
                        items = listOf(item.toTransferSource()),
                      )
                    }

                    FolderAction.Cut -> {
                      clipboard.setCut(
                        sourceSiteId = siteId,
                        items = listOf(item.toTransferSource()),
                      )
                    }

                    FolderAction.Delete -> {
                      deleteRequest = FolderDeleteRequest(
                        entityId = item.id,
                        folderName = item.name,
                        shouldPopOnSuccess = false,
                      )
                    }

                    FolderAction.SelectMultiple,
                    FolderAction.StartReorder -> Unit
                  }
                },
              )
            }
          },
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
            presenterScope.launch {
              model.moveChildEntity(
                entityId = commit.movedKey,
                parentEntityId = parentEntityId,
                lowerOrder = reorderOrders.lowerOrder,
                upperOrder = reorderOrders.upperOrder,
              )
                .withDefaultExceptionHandler(toast)
                .onException {
                  reorderState.resetToServerKeys(serverChildIds)
                }
              isPersistingReorder = false
            }
          },
        )

        pasteTarget?.let { resolvedPasteTarget ->
          Box(modifier = Modifier.align(Alignment.BottomCenter)) {
            AnimatedVisibility(
              visible = animatedPasteBarVisible,
              enter = fadeIn(animationSpec = tween(220)) + slideInVertically(
                animationSpec = tween(220),
                initialOffsetY = { it / 2 },
              ),
              exit = fadeOut(animationSpec = tween(180)) + slideOutVertically(
                animationSpec = tween(180),
                targetOffsetY = { it / 2 },
              ),
            ) {
              EntityPasteBar(
                bottomOffset = Route.Folder(entityId).toastBottomInset + FolderScreenPasteBarBottomOffset,
                loading = isPasting,
                onClear = { clipboard.clear() },
                onPaste = {
                  if (!isPasting) {
                    isPasting = true
                    presenterScope.launch {
                      clipboard.pasteInto(resolvedPasteTarget).collect(
                        onPending = { count ->
                          toast.show(ToastType.Loading, "${count}개의 항목을 붙여넣는 중이에요", kotlin.time.Duration.ZERO)
                        },
                        onSettled = { result ->
                          result
                            .withDefaultExceptionHandler(toast)
                            .onOk { count -> toast.show(ToastType.Success, "${count}개의 항목을 붙여넣었어요") }
                            .onErr { error -> toast.show(ToastType.Error, error.toMessage()) }
                        },
                      )
                      isPasting = false
                    }
                  }
                },
              )
            }
          }
        }
      }
    },
  )
}
