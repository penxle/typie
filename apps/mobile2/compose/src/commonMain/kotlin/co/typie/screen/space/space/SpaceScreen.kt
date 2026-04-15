package co.typie.screen.space.space

import androidx.compose.animation.core.animateDpAsState
import androidx.compose.animation.core.tween
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.WindowInsets
import androidx.compose.foundation.layout.asPaddingValues
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.runtime.Composable
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
import androidx.compose.ui.text.style.TextOverflow
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
import co.typie.domain.entitytransfer.EntityClipboardMode
import co.typie.domain.entitytransfer.EntityClipboardService
import co.typie.domain.entitytransfer.EntityPasteBar
import co.typie.domain.entitytransfer.EntityPasteTarget
import co.typie.domain.entitytransfer.toMessage
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
import co.typie.screen.space.document.DocumentViewModel
import co.typie.screen.space.entity.EntityCreateBottomBarAction
import co.typie.screen.space.entity.EntityCreateViewModel
import co.typie.screen.space.entity.EntitySelectionViewModel
import co.typie.screen.space.folder.FolderViewModel
import co.typie.shell.MainBottomBarPill
import co.typie.storage.Preference
import co.typie.ui.component.EntityBottomOverlayDefaults
import co.typie.ui.component.Screen
import co.typie.ui.component.SpacePopover
import co.typie.ui.component.SpacePopoverLeadingKey
import co.typie.ui.component.Text
import co.typie.ui.component.bottombar.BottomBarDefaults
import co.typie.ui.component.bottombar.ProvideBottomBar
import co.typie.ui.component.dialog.DialogResult
import co.typie.ui.component.dialog.LocalDialog
import co.typie.ui.component.dialog.confirm
import co.typie.ui.component.entitycontainer.EntityContainerBottomOverlayStack
import co.typie.ui.component.entitycontainer.EntityContainerEditAction
import co.typie.ui.component.entitycontainer.EntityContainerListContent
import co.typie.ui.component.entitycontainer.EntityContainerSelectionBar
import co.typie.ui.component.entitycontainer.EntityContainerTopBarTrailing
import co.typie.ui.component.entitycontainer.EntityContainerTopBarTrailingKey
import co.typie.ui.component.entitycontainer.calculateEntityContainerBottomOverlayMetrics
import co.typie.ui.component.entitycontainer.calculateEntityReorderOrdersFromOrderedKeys
import co.typie.ui.component.entitycontainer.displayOrderedEntityItems
import co.typie.ui.component.entitycontainer.rememberEntityContainerSelection
import co.typie.ui.component.entitycontainer.resolveEntityContainerTransferSources
import co.typie.ui.component.formatSpaceSummary
import co.typie.ui.component.reorder.rememberReorderableListState
import co.typie.ui.component.reorder.reorderableListContainer
import co.typie.ui.component.sheet.LocalSheet
import co.typie.ui.component.toast.LocalToast
import co.typie.ui.component.toast.ToastType
import co.typie.ui.component.topbar.ProvideTopBar
import co.typie.ui.component.topbar.TopBarDefaults
import co.typie.ui.component.topbar.topBarScrollOffset
import co.typie.ui.state.rememberScrollState
import co.typie.ui.theme.AppTheme
import kotlin.time.Duration
import kotlinx.coroutines.launch

@Composable
fun SpaceScreen() {
  val nav = Nav.current
  val haptic = LocalHapticFeedback.current
  val uriHandler = LocalUriHandler.current
  val sheet = LocalSheet.current
  val dialog = LocalDialog.current
  val toast = LocalToast.current
  val clipboard = EntityClipboardService
  val model = viewModel { SpaceViewModel() }
  val createActionModel = viewModel(key = "space-create-actions") { EntityCreateViewModel() }
  val folderActionModel = viewModel(key = "space-folder-actions") { FolderViewModel() }
  val documentActionModel = viewModel(key = "space-document-actions") { DocumentViewModel() }
  val selectionActionModel =
    viewModel(key = "space-selection-actions") { EntitySelectionViewModel() }
  val scrollState = rememberScrollState("space")
  val presenterScope = rememberCoroutineScope()
  var isReordering by remember { mutableStateOf(false) }
  var isPersistingReorder by remember { mutableStateOf(false) }
  var isPasting by remember { mutableStateOf(false) }
  var animatedPasteBarVisible by remember { mutableStateOf(false) }
  var overlayMetrics by remember {
    mutableStateOf(
      calculateEntityContainerBottomOverlayMetrics(
        baseBottomInset = BottomBarDefaults.BarAreaHeight,
        hasPasteBar = false,
        pasteBarHeight = EntityBottomOverlayDefaults.BarHeight,
        hasSelectionBar = false,
        selectionBarHeight = EntityBottomOverlayDefaults.BarHeight,
      )
    )
  }
  val siteId = model.siteId

  val site = (model.query.state as? QueryState.Success)?.data?.site
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
    remember(site) {
      site?.let { currentSite ->
        EntityPasteTarget(
          siteId = currentSite.id,
          destinationEntityId = null,
          destinationDepth = -1,
          ancestorFolderIds = emptySet(),
          lowerOrder = currentSite.entities.lastOrNull()?.order,
          upperOrder = null,
        )
      }
    }
  val isPasteBarVisible =
    clipboardState != null && pasteTarget != null && clipboard.canPaste(requireNotNull(pasteTarget))
  val isCurrentRoute = nav.current == LocalRoute.current
  val shouldShowPasteBar = isPasteBarVisible && isCurrentRoute
  var lastReservedBottomSpacerTarget by remember {
    mutableStateOf(overlayMetrics.reservedSpacerHeight)
  }
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
      label = "space-bottom-spacer-height",
    )
  SideEffect { lastReservedBottomSpacerTarget = overlayMetrics.reservedSpacerHeight }

  LaunchedEffect(shouldShowPasteBar) { animatedPasteBarVisible = shouldShowPasteBar }

  val serverEntities =
    remember(site?.entities) {
      normalizeSpaceEntities(siteName = site?.name.orEmpty(), entities = site?.entities.orEmpty())
    }
  val selection = rememberEntityContainerSelection(serverEntities)
  val selectionState = selection.state
  val serverEntityIds = remember(serverEntities) { serverEntities.map { it.id } }
  val selectionSummary = selection.summary
  val isSelectionBarVisible = selection.isSelectionBarVisible

  LaunchedEffect(site?.id) {
    isReordering = false
    selection.reset()
    isPersistingReorder = false
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
            siteId?.let { sourceSiteId ->
              clipboard.setCopy(
                sourceSiteId = sourceSiteId,
                items = resolveEntityContainerTransferSources(selectionSummary),
              )
              selection.reset()
            }
          },
          onCut = {
            siteId?.let { sourceSiteId ->
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

  val editActions =
    listOf(
      EntityContainerEditAction(
        icon = Lucide.SquareCheck,
        label = "여러 항목 선택하기",
        onClick = { startSelection() },
      ),
      EntityContainerEditAction(
        icon = Lucide.ChevronsUpDown,
        label = "순서 변경하기",
        onClick = {
          selection.reset()
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
    trailing =
      if (serverEntities.isEmpty()) null
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
    scrollOffset = scrollState.topBarScrollOffset(),
  )

  ProvideBottomBar(
    pill = { MainBottomBarPill() },
    action = {
      EntityCreateBottomBarAction(
        model = createActionModel,
        siteId = siteId,
        onCreated = { model.refetch() },
        onFolderCreated = { nav.navigate(Route.Folder(it)) },
        onDocumentCreated = { nav.navigate(Route.Editor(it)) },
      )
    },
  )

  Screen(loadable = model.query) { contentPadding ->
    val reorderViewportTopInset =
      maxOf(
        0.dp,
        contentPadding.calculateTopPadding() -
          TopBarDefaults.BlurFadeHeight -
          TopBarDefaults.ContentTopSpacing,
      )
    val reorderViewportBottomInset =
      WindowInsets.safeDrawing.asPaddingValues().calculateBottomPadding() + 72.dp

    val reorderState =
      rememberReorderableListState(keys = serverEntityIds, verticalScrollableState = scrollState)
    val displayEntities =
      remember(serverEntities, reorderState.displayedKeys) {
        displayOrderedEntityItems(serverEntities, reorderState.displayedKeys)
      }

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
        items = displayEntities,
        emptyMessage = "문서와 폴더가 여기 나타나요",
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
        header = {
          SpaceHeader(
            title = site?.name.orEmpty(),
            summary =
              formatSpaceSummary(
                folderCount = site?.folderCount ?: 0,
                documentCount = site?.documentCount ?: 0,
              ),
          )
        },
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
                            model = documentActionModel,
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
                            model = documentActionModel,
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
                          documentActionModel
                            .deleteDocument(item.documentId)
                            .withDefaultExceptionHandler(toast)
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
        onFolderClick = { entityId -> nav.navigate(Route.Folder(entityId)) },
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
                            model = folderActionModel,
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
                            model = folderActionModel,
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
                          folderActionModel
                            .deleteFolderEntity(item.id)
                            .withDefaultExceptionHandler(toast)
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
            if (commit == null || commit.orderedKeys == serverEntityIds) {
              return@onDragStopped
            }

            val reorderOrders =
              calculateEntityReorderOrdersFromOrderedKeys(
                items = serverEntities,
                orderedKeys = commit.orderedKeys,
                movedKey = commit.movedKey,
              )
                ?: run {
                  reorderState.resetToServerKeys(serverEntityIds)
                  return@onDragStopped
                }

            isPersistingReorder = true
            presenterScope.launch {
              model
                .moveRootEntity(
                  entityId = commit.movedKey,
                  lowerOrder = reorderOrders.lowerOrder,
                  upperOrder = reorderOrders.upperOrder,
                )
                .withDefaultExceptionHandler(toast)
                .onException { reorderState.resetToServerKeys(serverEntityIds) }
              isPersistingReorder = false
            }
          },
      )

      EntityContainerBottomOverlayStack(
        baseBottomInset = BottomBarDefaults.BarAreaHeight,
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

@Composable
private fun SpaceHeader(title: String, summary: String) {
  Column(
    modifier =
      Modifier.fillMaxWidth().padding(horizontal = 16.dp).padding(top = 4.dp, bottom = 24.dp)
  ) {
    Text(
      if (title.isBlank()) " " else title,
      style = AppTheme.typography.display,
      maxLines = 1,
      overflow = TextOverflow.Ellipsis,
    )

    Spacer(Modifier.height(8.dp))

    Text(summary, style = AppTheme.typography.body, color = AppTheme.colors.textTertiary)
  }
}
