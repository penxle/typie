package co.typie.screen.space.folder

import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.WindowInsets
import androidx.compose.foundation.layout.asPaddingValues
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
import androidx.compose.ui.platform.LocalUriHandler
import androidx.compose.ui.unit.dp
import androidx.lifecycle.viewmodel.compose.viewModel
import co.typie.domain.entity.DocumentEntityShareSheet
import co.typie.domain.entity.DocumentItemActionsSheet
import co.typie.domain.entity.DocumentRenameSheet
import co.typie.domain.entity.EntityAction
import co.typie.domain.entity.EntityContainerBottomOverlayStack
import co.typie.domain.entity.EntityContainerEditAction
import co.typie.domain.entity.EntityContainerListContent
import co.typie.domain.entity.EntityContainerSelectionBar
import co.typie.domain.entity.EntityContainerTopBarTrailing
import co.typie.domain.entity.EntityContainerTopBarTrailingKey
import co.typie.domain.entity.EntityIconPickerSheet
import co.typie.domain.entity.EntityIconPickerStops
import co.typie.domain.entity.EntityMoveSheet
import co.typie.domain.entity.EntityMoveStops
import co.typie.domain.entity.EntitySelectionActionsSheet
import co.typie.domain.entity.FolderEntityShareSheet
import co.typie.domain.entity.FolderItemActionsSheet
import co.typie.domain.entity.FolderRenameSheet
import co.typie.domain.entity.calculateEntityReorderOrdersFromOrderedKeys
import co.typie.domain.entity.displayEntityRows
import co.typie.domain.entity.document
import co.typie.domain.entity.folder
import co.typie.domain.entity.formatDocumentTitle
import co.typie.domain.entity.formatFolderMetadataSummary
import co.typie.domain.entity.formatFolderName
import co.typie.domain.entity.isRowEntity
import co.typie.domain.entity.rememberEntityContainerOverlayState
import co.typie.domain.entity.rememberEntityContainerSelection
import co.typie.domain.entitytransfer.EntityClipboardMode
import co.typie.domain.entitytransfer.EntityClipboardService
import co.typie.domain.entitytransfer.EntityPasteBar
import co.typie.domain.entitytransfer.EntityPasteTarget
import co.typie.domain.entitytransfer.toMessage
import co.typie.domain.entitytransfer.toTransferSource
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
import co.typie.screen.space.entity.EntityCreateBottomBarAction
import co.typie.screen.space.entity.EntityCreateViewModel
import co.typie.screen.space.entity.EntitySelectionViewModel
import co.typie.shell.MainBottomBarPill
import co.typie.storage.Preference
import co.typie.ui.component.ResponsiveContainerDefaults
import co.typie.ui.component.Screen
import co.typie.ui.component.bottombar.BottomBarDefaults
import co.typie.ui.component.bottombar.ProvideBottomBar
import co.typie.ui.component.dialog.DialogResult
import co.typie.ui.component.dialog.LocalDialog
import co.typie.ui.component.dialog.confirm
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
  val uriHandler = LocalUriHandler.current
  val sheet = LocalSheet.current
  val dialog = LocalDialog.current
  val presenterScope = rememberCoroutineScope()
  val toast = LocalToast.current
  val clipboard = EntityClipboardService
  val createActionModel = viewModel { EntityCreateViewModel() }
  val model = viewModel { FolderViewModel() }
  val selectionActionModel = viewModel { EntitySelectionViewModel() }
  val scrollState = rememberScrollState()
  var isReordering by remember { mutableStateOf(false) }
  var isPersistingReorder by remember { mutableStateOf(false) }
  var isPasting by remember { mutableStateOf(false) }
  val root = (model.query.state as? QueryState.Success)?.data?.entity
  val displayRoot = model.query.data.entity
  val entityDetails = root?.entityDetails_entity
  val displayEntityDetails = displayRoot.entityDetails_entity
  val topBarEntity = displayRoot.folderTopBar_entity
  val loading = root == null
  val entity = entityDetails?.entityRow_entity
  val displayEntity = displayEntityDetails.entityRow_entity
  val folder = entity?.folder
  val displayFolder = displayEntity.folder
  val folderTitle = displayFolder?.let { formatFolderName(it.name) } ?: "폴더"
  val folderMetadataSummary =
    formatFolderMetadataSummary(
      folderCount = displayFolder?.folderCount ?: 0,
      documentCount = displayFolder?.documentCount ?: 0,
      characterCount = displayEntityDetails.folder?.characterCount ?: 0,
    )
  val serverChildren =
    remember(root?.children) {
      root?.children.orEmpty().mapNotNull { child ->
        child.entityRow_entity.takeIf { it.isRowEntity() }
      }
    }
  val serverChildIds = remember(serverChildren) { serverChildren.map { it.id } }
  val reorderState =
    rememberReorderableListState(keys = serverChildIds, verticalScrollableState = scrollState)
  val displayChildren =
    remember(serverChildren, reorderState.displayedKeys) {
      displayEntityRows(serverChildren, reorderState.displayedKeys)
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
    remember(root) {
      root?.let { currentEntity ->
        val currentEntityRow = currentEntity.entityDetails_entity.entityRow_entity
        EntityPasteTarget(
          siteId = currentEntity.site.id,
          destinationEntityId = currentEntityRow.id,
          destinationDepth = currentEntityRow.depth,
          ancestorFolderIds =
            currentEntity.entityDetails_entity.ancestors.mapTo(mutableSetOf()) { it.id },
          lowerOrder = currentEntity.children.lastOrNull()?.entityRow_entity?.order,
          upperOrder = null,
        )
      }
    }
  val isPasteBarVisible =
    clipboardState != null && pasteTarget != null && clipboard.canPaste(requireNotNull(pasteTarget))
  val isCurrentRoute = nav.current == LocalRoute.current
  val shouldShowPasteBar = isPasteBarVisible && isCurrentRoute
  val isSelectionBarVisible = selection.isSelectionBarVisible
  val overlayBaseBottomInset = BottomBarDefaults.BarAreaHeight
  val overlayState =
    rememberEntityContainerOverlayState(
      baseBottomInset = overlayBaseBottomInset,
      pasteBarVisible = shouldShowPasteBar,
      resetKey = entityId,
    )

  fun startSelection(initialIds: Set<String> = emptySet()) {
    isReordering = false
    selection.start(initialIds)
  }

  fun presentFolderShare(entityIds: List<String>) {
    val resolvedEntityIds = entityIds.map(String::trim).filter(String::isNotEmpty)
    if (resolvedEntityIds.isEmpty()) {
      return
    }

    presenterScope.launch {
      sheet.present {
        FolderEntityShareSheet(entityIds = resolvedEntityIds, onUpdated = { model.refetch() })
      }
    }
  }

  fun presentDocumentShare(entityIds: List<String>) {
    val resolvedEntityIds = entityIds.map(String::trim).filter(String::isNotEmpty)
    if (resolvedEntityIds.isEmpty()) {
      return
    }

    presenterScope.launch {
      sheet.present {
        DocumentEntityShareSheet(entityIds = resolvedEntityIds, onUpdated = { model.refetch() })
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
          onShareFolders = { presentFolderShare(selectionSummary.folderItems.map { it.id }) },
          onShareDocuments = { presentDocumentShare(selectionSummary.documentItems.map { it.id }) },
          onCopy = {
            root?.site?.id?.let { sourceSiteId ->
              clipboard.setCopy(
                sourceSiteId = sourceSiteId,
                items = selectionSummary.selectedItems.map { it.toTransferSource() },
              )
              selection.reset()
            }
          },
          onCut = {
            root?.site?.id?.let { sourceSiteId ->
              clipboard.setCut(
                sourceSiteId = sourceSiteId,
                items = selectionSummary.selectedItems.map { it.toTransferSource() },
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

  fun onCenterAction(action: EntityAction) {
    when (action) {
      EntityAction.Rename -> {
        val resolvedFolder = folder ?: return
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

      EntityAction.ChangeIcon -> {
        val resolvedEntity = entity
        val resolvedFolder = folder
        if (resolvedEntity == null || resolvedFolder == null) {
          return
        }
        presenterScope.launch {
          sheet.present(stops = EntityIconPickerStops) {
            EntityIconPickerSheet(
              model = model,
              entityId = resolvedEntity.id,
              initialIcon = resolvedEntity.entityIcon_entity.icon,
              initialColor = resolvedEntity.entityIcon_entity.iconColor,
              defaultIconName = "folder",
            )
          }
        }
      }

      EntityAction.Share -> {
        val resolvedEntity = entity
        val resolvedFolder = folder
        if (resolvedEntity == null || resolvedFolder == null) {
          return
        }
        presentFolderShare(listOf(resolvedEntity.id))
      }

      EntityAction.Move -> {
        val resolvedEntity = entityDetails
        val resolvedFolder = folder
        if (resolvedEntity == null || resolvedFolder == null) {
          return
        }
        presenterScope.launch {
          sheet.present(stops = EntityMoveStops) {
            EntityMoveSheet(
              source = resolvedEntity.toTransferSource(),
              initialDestinationId = resolvedEntity.ancestors.lastOrNull()?.id,
            )
          }
        }
      }

      EntityAction.OpenExternal -> {
        entity?.url?.let(uriHandler::openUri)
      }

      EntityAction.StartReorder -> {
        selection.reset()
        isReordering = true
      }

      EntityAction.SelectMultiple -> {
        startSelection()
      }

      EntityAction.Copy -> {
        val resolvedEntity = entityDetails
        val sourceSiteId = root?.site?.id
        val resolvedFolder = folder
        if (resolvedEntity == null || resolvedFolder == null || sourceSiteId == null) {
          return
        }
        clipboard.setCopy(
          sourceSiteId = sourceSiteId,
          items = listOf(resolvedEntity.toTransferSource()),
        )
      }

      EntityAction.Cut -> {
        val resolvedEntity = entityDetails
        val sourceSiteId = root?.site?.id
        val resolvedFolder = folder
        if (resolvedEntity == null || resolvedFolder == null || sourceSiteId == null) {
          return
        }
        clipboard.setCut(
          sourceSiteId = sourceSiteId,
          items = listOf(resolvedEntity.toTransferSource()),
        )
      }

      EntityAction.Delete -> {
        val resolvedFolder = folder ?: return
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
        onClick = { onCenterAction(EntityAction.SelectMultiple) },
      ),
      EntityContainerEditAction(
        icon = Lucide.ChevronsUpDown,
        label = "순서 변경하기",
        onClick = { onCenterAction(EntityAction.StartReorder) },
      ),
    )

  ProvideTopBar(
    leading = { TopBarBackButton() },
    center = {
      Box(contentAlignment = Alignment.Center, modifier = Modifier.fillMaxWidth()) {
        FolderTopBarCenterMenu(
          title = folderTitle,
          subtitle = folderMetadataSummary,
          entity = topBarEntity,
          loading = loading,
          onAction = ::onCenterAction,
          modifier = Modifier.fillMaxWidth().widthIn(max = ResponsiveContainerDefaults.MaxWidth),
        )
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
        siteId = root?.site?.id ?: Preference.siteId,
        parentEntityId = entityId,
        onCreated = { model.refetch() },
        onFolderCreated = { nav.navigate(Route.Folder(it)) },
        onDocumentCreated = { nav.navigate(Route.Editor(it)) },
      )
    },
  )

  Screen(
    loadable = model.query,
    foregroundOverlay = {
      EntityContainerBottomOverlayStack(
        baseBottomInset = overlayBaseBottomInset,
        showSelectionBar = isSelectionBarVisible,
        showPasteBar = overlayState.animatedPasteBarVisible && pasteTarget != null,
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
        onMetricsChanged = overlayState::onMetricsChanged,
      )
    },
  ) { contentPadding ->
    val reorderViewportTopInset =
      maxOf(
        0.dp,
        contentPadding.calculateTopPadding() -
          TopBarDefaults.BlurFadeHeight -
          TopBarDefaults.ContentTopSpacing,
      )
    val reorderViewportBottomInset =
      WindowInsets.safeDrawing.asPaddingValues().calculateBottomPadding() + overlayBaseBottomInset

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
        bottomSpacerHeight = overlayState.reservedBottomSpacerHeight,
        modifier =
          Modifier.fillMaxSize()
            .verticalScroll(scrollState)
            .padding(contentPadding)
            .safeBottomPadding(),
        header = {},
        onDocumentClick = { childEntityId -> nav.navigate(Route.Editor(childEntityId)) },
        onDocumentLongPress = onDocumentLongPress@{ entity ->
            val document = entity.document ?: return@onDocumentLongPress
            if (selectionState.isSelecting) {
              if (entity.id in selectionState.selectedIds) {
                openSelectionActions()
              } else {
                selection.toggle(entity.id)
              }
            } else {
              presenterScope.launch {
                sheet.present {
                  DocumentItemActionsSheet(entity = entity) { action ->
                    when (action) {
                      EntityAction.Rename -> {
                        presenterScope.launch {
                          sheet.present {
                            DocumentRenameSheet(
                              model = model,
                              documentId = document.id,
                              initialTitle = document.title,
                            )
                          }
                        }
                      }

                      EntityAction.ChangeIcon -> {
                        presenterScope.launch {
                          sheet.present(stops = EntityIconPickerStops) {
                            EntityIconPickerSheet(
                              model = model,
                              entityId = entity.id,
                              initialIcon = entity.entityIcon_entity.icon,
                              initialColor = entity.entityIcon_entity.iconColor,
                              defaultIconName = "file",
                            )
                          }
                        }
                      }

                      EntityAction.OpenExternal -> uriHandler.openUri(entity.url)

                      EntityAction.Share -> presentDocumentShare(listOf(entity.id))

                      EntityAction.Move -> {
                        presenterScope.launch {
                          sheet.present(stops = EntityMoveStops) {
                            EntityMoveSheet(
                              source = entity.toTransferSource(),
                              initialDestinationId = entityId,
                            )
                          }
                        }
                      }

                      EntityAction.Copy -> {
                        clipboard.setCopy(
                          sourceSiteId = Preference.siteId!!,
                          items = listOf(entity.toTransferSource()),
                        )
                      }

                      EntityAction.Cut -> {
                        clipboard.setCut(
                          sourceSiteId = Preference.siteId!!,
                          items = listOf(entity.toTransferSource()),
                        )
                      }

                      EntityAction.Delete -> {
                        presenterScope.launch {
                          val result =
                            dialog.confirm(
                              title = "문서 삭제",
                              message =
                                "\"${formatDocumentTitle(document.title)}\" 문서를 삭제하시겠어요? 삭제 후 30일 동안 휴지통에 보관돼요.",
                              confirmText = "삭제하기",
                              confirmIsDestructive = true,
                            )
                          if (result is DialogResult.Resolved) {
                            model.deleteDocument(document.id).withDefaultExceptionHandler(toast)
                          }
                        }
                      }

                      EntityAction.SelectMultiple -> Unit

                      EntityAction.StartReorder -> {
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
        onFolderLongPress = onFolderLongPress@{ entity ->
            val folder = entity.folder ?: return@onFolderLongPress
            if (selectionState.isSelecting) {
              if (entity.id in selectionState.selectedIds) {
                openSelectionActions()
              } else {
                selection.toggle(entity.id)
              }
            } else {
              presenterScope.launch {
                sheet.present {
                  FolderItemActionsSheet(entity = entity) { action ->
                    when (action) {
                      EntityAction.Rename -> {
                        presenterScope.launch {
                          sheet.present {
                            FolderRenameSheet(
                              model = model,
                              folderId = folder.id,
                              initialName = folder.name,
                            )
                          }
                        }
                      }

                      EntityAction.ChangeIcon -> {
                        presenterScope.launch {
                          sheet.present(stops = EntityIconPickerStops) {
                            EntityIconPickerSheet(
                              model = model,
                              entityId = entity.id,
                              initialIcon = entity.entityIcon_entity.icon,
                              initialColor = entity.entityIcon_entity.iconColor,
                              defaultIconName = "folder",
                            )
                          }
                        }
                      }

                      EntityAction.OpenExternal -> uriHandler.openUri(entity.url)

                      EntityAction.Share -> presentFolderShare(listOf(entity.id))

                      EntityAction.Move -> {
                        presenterScope.launch {
                          sheet.present(stops = EntityMoveStops) {
                            EntityMoveSheet(
                              source = entity.toTransferSource(),
                              initialDestinationId = entityId,
                            )
                          }
                        }
                      }

                      EntityAction.Copy -> {
                        clipboard.setCopy(
                          sourceSiteId = Preference.siteId!!,
                          items = listOf(entity.toTransferSource()),
                        )
                      }

                      EntityAction.Cut -> {
                        clipboard.setCut(
                          sourceSiteId = Preference.siteId!!,
                          items = listOf(entity.toTransferSource()),
                        )
                      }

                      EntityAction.Delete -> {
                        presenterScope.launch {
                          val result =
                            dialog.confirm(
                              title = "폴더 삭제",
                              message =
                                "\"${formatFolderName(folder.name)}\" 폴더를 삭제하시겠어요? 삭제 후 30일 동안 휴지통에 보관돼요.",
                              confirmText = "삭제하기",
                              confirmIsDestructive = true,
                            )
                          if (result is DialogResult.Resolved) {
                            model.deleteFolderEntity(entity.id).withDefaultExceptionHandler(toast)
                          }
                        }
                      }

                      EntityAction.SelectMultiple -> Unit

                      EntityAction.StartReorder -> {
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
        onDragStopped = onDragStopped@{ commit ->
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
    }
  }
}
