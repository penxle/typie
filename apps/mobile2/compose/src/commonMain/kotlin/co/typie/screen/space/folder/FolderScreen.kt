package co.typie.screen.space.folder

import androidx.compose.animation.core.animateDpAsState
import androidx.compose.animation.core.tween
import androidx.compose.foundation.layout.Box
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
import androidx.compose.runtime.SideEffect
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
import androidx.lifecycle.viewmodel.compose.viewModel
import co.typie.domain.entity.DocumentItemActionsSheet
import co.typie.domain.entity.DocumentRenameSheet
import co.typie.domain.entity.EntityIconPickerSheet
import co.typie.domain.entity.EntityIconPickerStops
import co.typie.domain.entity.EntityMoveSheet
import co.typie.domain.entity.EntityMoveStops
import co.typie.domain.entity.EntitySelectionActionsSheet
import co.typie.domain.entity.EntityShareSheet
import co.typie.domain.entity.FolderAction
import co.typie.domain.entity.FolderItemActionsSheet
import co.typie.domain.entity.FolderRenameSheet
import co.typie.domain.entity.toTransferSource
import co.typie.domain.entity_transfer.EntityClipboardMode
import co.typie.domain.entity_transfer.EntityClipboardService
import co.typie.domain.entity_transfer.EntityPasteBar
import co.typie.domain.entity_transfer.EntityPasteTarget
import co.typie.domain.entity_transfer.EntityTransferSource
import co.typie.domain.entity_transfer.toMessage
import co.typie.ext.safeBottomPadding
import co.typie.ext.safeDrawing
import co.typie.ext.verticalScroll
import co.typie.graphql.QueryState
import co.typie.icons.Lucide
import co.typie.navigation.LocalRoute
import co.typie.navigation.Nav
import co.typie.result.onErr
import co.typie.result.onException
import co.typie.result.onOk
import co.typie.result.withDefaultExceptionHandler
import co.typie.route.Route
import co.typie.route.toastBottomInset
import co.typie.screen.space.entity.EntityCreateBottomBarAction
import co.typie.screen.space.entity.EntityCreateViewModel
import co.typie.screen.space.entity.EntitySelectionViewModel
import co.typie.shell.MainBottomBarPill
import co.typie.storage.Preference
import co.typie.ui.component.EntityBottomOverlayDefaults
import co.typie.ui.component.ResponsiveContainerDefaults
import co.typie.ui.component.Screen
import co.typie.ui.component.bottombar.ProvideBottomBar
import co.typie.ui.component.dialog.DialogResult
import co.typie.ui.component.dialog.LocalDialog
import co.typie.ui.component.dialog.confirm
import co.typie.ui.component.dialog.error
import co.typie.ui.component.entity_container.EntityContainerBottomOverlayStack
import co.typie.ui.component.entity_container.EntityContainerEditAction
import co.typie.ui.component.entity_container.EntityContainerListContent
import co.typie.ui.component.entity_container.EntityContainerSelectionBar
import co.typie.ui.component.entity_container.EntityContainerTopBarTrailing
import co.typie.ui.component.entity_container.EntityContainerTopBarTrailingKey
import co.typie.ui.component.entity_container.calculateEntityContainerBottomOverlayMetrics
import co.typie.ui.component.entity_container.calculateEntityReorderOrdersFromOrderedKeys
import co.typie.ui.component.entity_container.displayOrderedEntityItems
import co.typie.ui.component.entity_container.rememberEntityContainerSelection
import co.typie.ui.component.entity_container.resolveEntityContainerTransferSources
import co.typie.ui.component.reorder.rememberReorderableListState
import co.typie.ui.component.reorder.reorderableListContainer
import co.typie.ui.component.sheet.LocalSheet
import co.typie.ui.component.toast.LocalToast
import co.typie.ui.component.toast.ToastType
import co.typie.ui.component.topbar.ProvideTopBar
import co.typie.ui.component.topbar.TopBarBackButton
import co.typie.ui.component.topbar.TopBarDefaults
import co.typie.ui.state.rememberScrollState
import kotlin.time.Duration
import kotlinx.coroutines.launch

@Composable
fun FolderScreen(entityId: String) {
  val nav = Nav.current
  val haptic = LocalHapticFeedback.current
  val uriHandler = LocalUriHandler.current
  val sheet = LocalSheet.current
  val dialog = LocalDialog.current
  val presenterScope = rememberCoroutineScope()
  val toast = LocalToast.current
  val clipboard = EntityClipboardService
  val createActionModel =
    viewModel(key = "folder-create-actions:$entityId") { EntityCreateViewModel() }
  val model = viewModel(key = "folder:$entityId") { FolderViewModel() }
  val selectionActionModel =
    viewModel(key = "folder-selection-actions:$entityId") { EntitySelectionViewModel() }
  val scrollState = rememberScrollState("folder-scroll:$entityId")
  var isReordering by remember { mutableStateOf(false) }
  var isPersistingReorder by remember { mutableStateOf(false) }
  var isPasting by remember { mutableStateOf(false) }
  var animatedPasteBarVisible by remember { mutableStateOf(false) }
  var overlayMetrics by
    remember(entityId) {
      mutableStateOf(
        calculateEntityContainerBottomOverlayMetrics(
          baseBottomInset = Route.Folder(entityId).toastBottomInset,
          hasPasteBar = false,
          pasteBarHeight = EntityBottomOverlayDefaults.BarHeight,
          hasSelectionBar = false,
          selectionBarHeight = EntityBottomOverlayDefaults.BarHeight,
        )
      )
    }
  val entity = (model.query.state as? QueryState.Success)?.data?.entity
  val folder = entity?.node?.onFolder
  val folderTitle = folder?.name ?: "폴더"
  val folderSummary =
    co.typie.ui.component.formatFolderSummary(
      folderCount = folder?.folderCount ?: 0,
      documentCount = folder?.documentCount ?: 0,
    )
  val folderMetadataSummary =
    co.typie.ui.component.formatFolderMetadataSummary(
      folderCount = folder?.folderCount ?: 0,
      documentCount = folder?.documentCount ?: 0,
      characterCount = folder?.characterCount ?: 0,
    )
  val breadcrumbNames = buildList {
    add(entity?.site?.name ?: "내 스페이스")
    addAll(entity?.ancestors?.mapNotNull { it.node.onFolder?.name }.orEmpty())
  }
  val serverChildren =
    remember(entity?.children) {
      normalizeFolderChildren(
        siteName = entity?.site?.name.orEmpty(),
        children = entity?.children.orEmpty(),
      )
    }
  val serverChildIds = remember(serverChildren) { serverChildren.map { it.id } }
  val reorderState =
    rememberReorderableListState(keys = serverChildIds, verticalScrollableState = scrollState)
  val displayChildren =
    remember(serverChildren, reorderState.displayedKeys) {
      displayOrderedEntityItems(serverChildren, reorderState.displayedKeys)
    }
  val selection = rememberEntityContainerSelection(displayChildren)
  val selectionState = selection.state
  val selectionSummary = selection.summary

  LaunchedEffect(entityId) {
    model.entityId = entityId
    isReordering = false
    selection.reset()
    isPersistingReorder = false
  }
  val clipboardState = clipboard.state
  val cutDimmedItemIds =
    remember(clipboardState) {
      if (clipboardState?.mode == EntityClipboardMode.Cut) {
        clipboardState.items.mapTo(mutableSetOf()) { it.id }
      } else {
        emptySet()
      }
    }
  val pasteTarget =
    remember(entity) {
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
  val isPasteBarVisible =
    clipboardState != null && pasteTarget != null && clipboard.canPaste(requireNotNull(pasteTarget))
  val isCurrentRoute = nav.current == LocalRoute.current
  val shouldShowPasteBar = isPasteBarVisible && isCurrentRoute
  val isSelectionBarVisible = selection.isSelectionBarVisible
  var lastReservedBottomSpacerTarget by
    remember(entityId) { mutableStateOf(overlayMetrics.reservedSpacerHeight) }
  val reservedBottomSpacerAnimationDuration =
    if (overlayMetrics.reservedSpacerHeight < lastReservedBottomSpacerTarget) {
      EntityBottomOverlayDefaults.ExitDurationMillis
    } else {
      EntityBottomOverlayDefaults.EnterDurationMillis
    }
  val reservedBottomSpacerHeight by
    animateDpAsState(
      targetValue = overlayMetrics.reservedSpacerHeight,
      animationSpec = tween(reservedBottomSpacerAnimationDuration),
      label = "folder-bottom-spacer-height",
    )
  SideEffect { lastReservedBottomSpacerTarget = overlayMetrics.reservedSpacerHeight }

  LaunchedEffect(shouldShowPasteBar) { animatedPasteBarVisible = shouldShowPasteBar }
  if (isCurrentRoute) {
    SideEffect { toast.bottomInset = overlayMetrics.toastBottomInset }
  }

  DisposableEffect(entityId) {
    onDispose { toast.bottomInset = Route.Folder(entityId).toastBottomInset }
  }

  fun startSelection(initialIds: Set<String> = emptySet()) {
    isReordering = false
    selection.start(initialIds)
  }

  fun presentShare(entityIds: List<String>) {
    val resolvedEntityIds = entityIds.map(String::trim).filter(String::isNotEmpty)
    if (resolvedEntityIds.isEmpty()) {
      return
    }

    presenterScope.launch {
      sheet.present {
        EntityShareSheet(entityIds = resolvedEntityIds, onUpdated = { model.refetch() })
      }
    }
  }

  fun openSelectionActions() {
    if (selectionSummary.selectedItems.isEmpty()) {
      return
    }

    presenterScope.launch {
      sheet.present {
        EntitySelectionActionsSheet(
          summary = selectionSummary,
          onChangeIcon = {
            presenterScope.launch {
              sheet.present(stops = EntityIconPickerStops) {
                EntityIconPickerSheet(
                  model = selectionActionModel,
                  entityIds = selectionSummary.selectedItems.map { it.id },
                  initialIcon = selectionSummary.commonIconName,
                  initialColor = selectionSummary.commonIconColor,
                )
              }
            }
          },
          onShareFolders = { presentShare(selectionSummary.folderItems.map { it.id }) },
          onShareDocuments = { presentShare(selectionSummary.documentItems.map { it.id }) },
          onCopy = {
            entity?.site?.id?.let { sourceSiteId ->
              clipboard.setCopy(
                sourceSiteId = sourceSiteId,
                items = resolveEntityContainerTransferSources(selectionSummary),
              )
              selection.reset()
            }
          },
          onCut = {
            entity?.site?.id?.let { sourceSiteId ->
              clipboard.setCut(
                sourceSiteId = sourceSiteId,
                items = resolveEntityContainerTransferSources(selectionSummary),
              )
              selection.reset()
            }
          },
          onDelete = {
            presenterScope.launch {
              val entityIds = selectionSummary.selectedItems.map { it.id }
              val result =
                dialog.confirm(
                  title = "선택한 항목 삭제",
                  message = "선택한 ${entityIds.size}개 항목을 삭제하시겠어요? 삭제 후 30일 동안 휴지통에 보관돼요.",
                  confirmText = "삭제하기",
                  confirmIsDestructive = true,
                )
              if (result is DialogResult.Resolved) {
                selectionActionModel
                  .deleteEntities(entityIds)
                  .withDefaultExceptionHandler(toast)
                  .onOk {
                    toast.show(ToastType.Success, "${entityIds.size}개의 항목을 삭제했어요")
                    selection.reset()
                  }
              }
            }
          },
        )
      }
    }
  }

  fun onCenterAction(action: FolderAction, closePopover: () -> Unit) {
    when (action) {
      FolderAction.Rename -> {
        val resolvedFolder = folder ?: return closePopover()
        closePopover()
        presenterScope.launch {
          sheet.present {
            FolderRenameSheet(
              model = model,
              folderId = resolvedFolder.id,
              initialName = resolvedFolder.name,
            )
          }
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
        presenterScope.launch {
          sheet.present(stops = EntityIconPickerStops) {
            EntityIconPickerSheet(
              model = model,
              entityId = resolvedEntity.id,
              initialIcon = resolvedEntity.icon,
              initialColor = resolvedEntity.iconColor,
              defaultIconName = "folder",
            )
          }
        }
      }

      FolderAction.Share -> {
        val resolvedEntity = entity
        val resolvedFolder = folder
        if (resolvedEntity == null || resolvedFolder == null) {
          closePopover()
          return
        }
        closePopover()
        presentShare(listOf(resolvedEntity.id))
      }

      FolderAction.Move -> {
        val resolvedEntity = entity
        val resolvedFolder = folder
        if (resolvedEntity == null || resolvedFolder == null) {
          closePopover()
          return
        }
        closePopover()
        presenterScope.launch {
          sheet.present(stops = EntityMoveStops) {
            EntityMoveSheet(
              source =
                EntityTransferSource.Folder(
                  id = resolvedEntity.id,
                  title = resolvedFolder.name,
                  depth = resolvedEntity.depth,
                  maxDescendantFoldersDepth = resolvedFolder.maxDescendantFoldersDepth,
                )
            )
          }
        }
      }

      FolderAction.OpenExternal -> {
        closePopover()
        entity?.url?.let(uriHandler::openUri)
      }

      FolderAction.StartReorder -> {
        closePopover()
        selection.reset()
        isReordering = true
      }

      FolderAction.SelectMultiple -> {
        closePopover()
        startSelection()
      }

      FolderAction.Copy -> {
        val resolvedEntity = entity
        val resolvedFolder = folder
        if (resolvedEntity == null || resolvedFolder == null) {
          closePopover()
          return
        }
        clipboard.setCopy(
          sourceSiteId = resolvedEntity.site.id,
          items =
            listOf(
              EntityTransferSource.Folder(
                id = resolvedEntity.id,
                title = resolvedFolder.name,
                depth = resolvedEntity.depth,
                maxDescendantFoldersDepth = resolvedFolder.maxDescendantFoldersDepth,
              )
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
          items =
            listOf(
              EntityTransferSource.Folder(
                id = resolvedEntity.id,
                title = resolvedFolder.name,
                depth = resolvedEntity.depth,
                maxDescendantFoldersDepth = resolvedFolder.maxDescendantFoldersDepth,
              )
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
        presenterScope.launch {
          val result =
            dialog.confirm(
              title = "폴더 삭제",
              message = "\"${resolvedFolder.name}\" 폴더를 삭제하시겠어요? 삭제 후 30일 동안 휴지통에 보관돼요.",
              confirmText = "삭제하기",
              confirmIsDestructive = true,
            )
          if (result is DialogResult.Resolved) {
            model.deleteFolderEntity(entityId).withDefaultExceptionHandler(toast).onOk { nav.pop() }
          }
        }
      }
    }
  }

  val editActions =
    listOf(
      EntityContainerEditAction(
        icon = Lucide.SquareCheck,
        label = "여러 항목 선택하기",
        onClick = { closePopover -> onCenterAction(FolderAction.SelectMultiple, closePopover) },
      ),
      EntityContainerEditAction(
        icon = Lucide.ChevronsUpDown,
        label = "순서 변경하기",
        onClick = { closePopover -> onCenterAction(FolderAction.StartReorder, closePopover) },
      ),
    )

  ProvideTopBar(
    leading = { TopBarBackButton() },
    center = {
      Box(contentAlignment = Alignment.Center, modifier = Modifier.fillMaxWidth()) {
        if (isReordering) {
          FolderTopBarCapsule(
            title = folderTitle,
            subtitle = folderSummary,
            iconName = entity?.icon,
            iconColor = entity?.iconColor,
            modifier = Modifier.fillMaxWidth().widthIn(max = ResponsiveContainerDefaults.MaxWidth),
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
            modifier = Modifier.fillMaxWidth().widthIn(max = ResponsiveContainerDefaults.MaxWidth),
          )
        }
      }
    },
    trailingKey = EntityContainerTopBarTrailingKey,
    trailing =
      if (displayChildren.isEmpty()) null
      else {
        {
          EntityContainerTopBarTrailing(
            isReordering = isReordering,
            isSelecting = selectionState.isSelecting,
            actions = editActions,
            onDoneClick = { isReordering = false },
            onCloseSelectionClick = { selection.reset() },
          )
        }
      },
  )

  ProvideBottomBar(
    pill = { MainBottomBarPill() },
    action = {
      EntityCreateBottomBarAction(
        model = createActionModel,
        siteId = entity?.site?.id ?: Preference.siteId,
        parentEntityId = entityId,
        onCreated = { model.refetch() },
        onFolderCreated = { nav.navigate(Route.Folder(it)) },
        onDocumentCreated = { nav.navigate(Route.Editor(it)) },
      )
    },
  )

  LaunchedEffect(model.query.state) {
    if (model.query.state is QueryState.Error) {
      dialog.error(nav = nav, onRetry = { model.refetch() })
    }
  }

  Screen(loading = model.query.state !is QueryState.Success, contentPadding = PaddingValues()) {
    contentPadding ->
    val reorderViewportTopInset =
      maxOf(
        0.dp,
        contentPadding.calculateTopPadding() -
          TopBarDefaults.BlurFadeHeight -
          TopBarDefaults.ContentTopSpacing,
      )
    val reorderViewportBottomInset =
      WindowInsets.safeDrawing.asPaddingValues().calculateBottomPadding() + 72.dp

    Box(
      modifier =
        Modifier.fillMaxSize()
          .reorderableListContainer(
            state = reorderState,
            viewportTopInset = reorderViewportTopInset,
            viewportBottomInset = reorderViewportBottomInset,
          )
    ) {
      EntityContainerListContent(
        items = displayChildren,
        emptyMessage = "폴더가 비어 있어요",
        isReordering = isReordering,
        reorderState = reorderState,
        isPersistingReorder = isPersistingReorder,
        selectionState = selectionState,
        dimmedItemIds = cutDimmedItemIds,
        bottomSpacerHeight = reservedBottomSpacerHeight,
        modifier =
          Modifier.fillMaxSize()
            .verticalScroll(scrollState)
            .padding(contentPadding)
            .safeBottomPadding(),
        header = {},
        onDocumentClick = { slug -> nav.navigate(Route.Editor(slug)) },
        onDocumentLongPress = { item ->
          if (selectionState.isSelecting) {
            if (item.id in selectionState.selectedIds) {
              openSelectionActions()
            } else {
              selection.toggle(item.id)
            }
          } else {
            presenterScope.launch {
              sheet.present {
                DocumentItemActionsSheet(item) { action ->
                  when (action) {
                    FolderAction.Rename -> {
                      presenterScope.launch {
                        sheet.present {
                          DocumentRenameSheet(
                            model = model,
                            documentId = item.documentId,
                            initialTitle = item.title,
                          )
                        }
                      }
                    }

                    FolderAction.ChangeIcon -> {
                      presenterScope.launch {
                        sheet.present(stops = EntityIconPickerStops) {
                          EntityIconPickerSheet(
                            model = model,
                            entityId = item.id,
                            initialIcon = item.iconName,
                            initialColor = item.iconColor,
                            defaultIconName = "file",
                          )
                        }
                      }
                    }

                    FolderAction.OpenExternal -> uriHandler.openUri(item.url)

                    FolderAction.Share -> presentShare(listOf(item.id))

                    FolderAction.Move -> {
                      presenterScope.launch {
                        sheet.present(stops = EntityMoveStops) {
                          EntityMoveSheet(source = item.toTransferSource())
                        }
                      }
                    }

                    FolderAction.Copy -> {
                      clipboard.setCopy(
                        sourceSiteId = Preference.siteId!!,
                        items = listOf(item.toTransferSource()),
                      )
                    }

                    FolderAction.Cut -> {
                      clipboard.setCut(
                        sourceSiteId = Preference.siteId!!,
                        items = listOf(item.toTransferSource()),
                      )
                    }

                    FolderAction.Delete -> {
                      presenterScope.launch {
                        val result =
                          dialog.confirm(
                            title = "문서 삭제",
                            message = "\"${item.title}\" 문서를 삭제하시겠어요? 삭제 후 30일 동안 휴지통에 보관돼요.",
                            confirmText = "삭제하기",
                            confirmIsDestructive = true,
                          )
                        if (result is DialogResult.Resolved) {
                          model.deleteDocument(item.documentId).withDefaultExceptionHandler(toast)
                        }
                      }
                    }

                    FolderAction.SelectMultiple -> Unit

                    FolderAction.StartReorder -> {
                      selection.reset()
                      isReordering = true
                    }
                  }
                }
              }
            }
          }
        },
        onFolderClick = { childEntityId -> nav.navigate(Route.Folder(childEntityId)) },
        onFolderLongPress = { item ->
          if (selectionState.isSelecting) {
            if (item.id in selectionState.selectedIds) {
              openSelectionActions()
            } else {
              selection.toggle(item.id)
            }
          } else {
            presenterScope.launch {
              sheet.present {
                FolderItemActionsSheet(item) { action ->
                  when (action) {
                    FolderAction.Rename -> {
                      presenterScope.launch {
                        sheet.present {
                          FolderRenameSheet(
                            model = model,
                            folderId = item.folderId,
                            initialName = item.name,
                          )
                        }
                      }
                    }

                    FolderAction.ChangeIcon -> {
                      presenterScope.launch {
                        sheet.present(stops = EntityIconPickerStops) {
                          EntityIconPickerSheet(
                            model = model,
                            entityId = item.id,
                            initialIcon = item.iconName,
                            initialColor = item.iconColor,
                            defaultIconName = "folder",
                          )
                        }
                      }
                    }

                    FolderAction.OpenExternal -> uriHandler.openUri(item.url)

                    FolderAction.Share -> presentShare(listOf(item.id))

                    FolderAction.Move -> {
                      presenterScope.launch {
                        sheet.present(stops = EntityMoveStops) {
                          EntityMoveSheet(source = item.toTransferSource())
                        }
                      }
                    }

                    FolderAction.Copy -> {
                      clipboard.setCopy(
                        sourceSiteId = Preference.siteId!!,
                        items = listOf(item.toTransferSource()),
                      )
                    }

                    FolderAction.Cut -> {
                      clipboard.setCut(
                        sourceSiteId = Preference.siteId!!,
                        items = listOf(item.toTransferSource()),
                      )
                    }

                    FolderAction.Delete -> {
                      presenterScope.launch {
                        val result =
                          dialog.confirm(
                            title = "폴더 삭제",
                            message = "\"${item.name}\" 폴더를 삭제하시겠어요? 삭제 후 30일 동안 휴지통에 보관돼요.",
                            confirmText = "삭제하기",
                            confirmIsDestructive = true,
                          )
                        if (result is DialogResult.Resolved) {
                          model.deleteFolderEntity(item.id).withDefaultExceptionHandler(toast)
                        }
                      }
                    }

                    FolderAction.SelectMultiple -> Unit

                    FolderAction.StartReorder -> {
                      selection.reset()
                      isReordering = true
                    }
                  }
                }
              }
            }
          }
        },
        onSelectionToggle = { selection.toggle(it) },
        onDragStarted = {
          haptic.performHapticFeedback(HapticFeedbackType.GestureThresholdActivate)
        },
        onDragMoved = { haptic.performHapticFeedback(HapticFeedbackType.SegmentFrequentTick) },
        onDragStopped = onDragStopped@{ commit ->
            haptic.performHapticFeedback(HapticFeedbackType.GestureEnd)
            if (commit == null || commit.orderedKeys == serverChildIds) {
              return@onDragStopped
            }

            val parentEntityId =
              entity?.id
                ?: run {
                  reorderState.resetToServerKeys(serverChildIds)
                  return@onDragStopped
                }
            val reorderOrders =
              calculateEntityReorderOrdersFromOrderedKeys(
                items = serverChildren,
                orderedKeys = commit.orderedKeys,
                movedKey = commit.movedKey,
              )
                ?: run {
                  reorderState.resetToServerKeys(serverChildIds)
                  return@onDragStopped
                }

            isPersistingReorder = true
            presenterScope.launch {
              model
                .moveChildEntity(
                  entityId = commit.movedKey,
                  parentEntityId = parentEntityId,
                  lowerOrder = reorderOrders.lowerOrder,
                  upperOrder = reorderOrders.upperOrder,
                )
                .withDefaultExceptionHandler(toast)
                .onException { reorderState.resetToServerKeys(serverChildIds) }
              isPersistingReorder = false
            }
          },
      )

      EntityContainerBottomOverlayStack(
        baseBottomInset = Route.Folder(entityId).toastBottomInset,
        showSelectionBar = isSelectionBarVisible,
        showPasteBar = animatedPasteBarVisible && pasteTarget != null,
        modifier = Modifier.align(Alignment.BottomCenter),
        selectionBar = {
          EntityContainerSelectionBar(
            selectedCount = selectionSummary.selectedItems.size,
            onClearSelection = { selection.clear() },
            onMoreClick = { openSelectionActions() },
          )
        },
        pasteBar = {
          pasteTarget?.let { resolvedPasteTarget ->
            EntityPasteBar(
              loading = isPasting,
              onClear = { clipboard.clear() },
              onPaste = {
                if (!isPasting) {
                  isPasting = true
                  presenterScope.launch {
                    clipboard
                      .pasteInto(resolvedPasteTarget)
                      .collect(
                        onPending = { count ->
                          toast.show(ToastType.Loading, "${count}개의 항목을 붙여넣는 중이에요", Duration.ZERO)
                        },
                        onSettled = { result ->
                          result
                            .withDefaultExceptionHandler(toast)
                            .onOk { count ->
                              toast.show(ToastType.Success, "${count}개의 항목을 붙여넣었어요")
                            }
                            .onErr { error -> toast.show(ToastType.Error, error.toMessage()) }
                        },
                      )
                    isPasting = false
                  }
                }
              },
            )
          }
        },
        onMetricsChanged = { overlayMetrics = it },
      )
    }
  }
}
