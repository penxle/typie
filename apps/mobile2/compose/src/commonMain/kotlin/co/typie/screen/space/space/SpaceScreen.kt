package co.typie.screen.space.space

import androidx.compose.animation.AnimatedVisibility
import androidx.compose.animation.core.tween
import androidx.compose.animation.fadeIn
import androidx.compose.animation.fadeOut
import androidx.compose.animation.slideInVertically
import androidx.compose.animation.slideOutVertically
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.WindowInsets
import androidx.compose.foundation.layout.asPaddingValues
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
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
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import androidx.lifecycle.viewmodel.compose.viewModel
import co.typie.entity_transfer.EntityClipboardMode
import co.typie.entity_transfer.EntityClipboardService
import co.typie.entity_transfer.EntityPasteBar
import co.typie.entity_transfer.EntityPasteTarget
import co.typie.entity_transfer.entityPasteBarToastBottomInset
import co.typie.entity_transfer.toMessage
import co.typie.ext.safeBottomPadding
import co.typie.ext.safeDrawing
import co.typie.ext.verticalScroll
import co.typie.graphql.QueryState
import co.typie.icons.Lucide
import co.typie.navigation.LocalRoute
import co.typie.navigation.Nav
import co.typie.overlay.LocalToast
import co.typie.overlay.ToastType
import co.typie.overlay.entityMoveSheet
import co.typie.result.onErr
import co.typie.result.onException
import co.typie.result.onOk
import co.typie.result.withDefaultExceptionHandler
import co.typie.route.Route
import co.typie.route.toastBottomInset
import co.typie.screen.space.document.DocumentDeleteRequest
import co.typie.screen.space.document.DocumentViewModel
import co.typie.screen.space.document.documentItemActionsSheet
import co.typie.screen.space.document.documentRenameSheet
import co.typie.screen.space.document.toTransferSource as toDocumentTransferSource
import co.typie.screen.space.entity.entityIconPickerSheet
import co.typie.screen.space.folder.FolderAction
import co.typie.screen.space.folder.FolderDeleteRequest
import co.typie.screen.space.folder.FolderViewModel
import co.typie.screen.space.folder.folderItemActionsSheet
import co.typie.screen.space.folder.folderRenameSheet
import co.typie.screen.space.folder.folderShareSheet
import co.typie.screen.space.folder.toTransferSource
import co.typie.shell.MainBottomBarPill
import co.typie.shell.SpaceBottomBarActionButton
import co.typie.storage.Preference
import co.typie.ui.component.ConfirmModal
import co.typie.ui.component.ErrorDialog
import co.typie.ui.component.Screen
import co.typie.ui.component.SpacePopover
import co.typie.ui.component.SpacePopoverLeadingKey
import co.typie.ui.component.Text
import co.typie.ui.component.bottombar.ProvideBottomBar
import co.typie.ui.component.entity_container.EntityContainerEditAction
import co.typie.ui.component.entity_container.EntityContainerListContent
import co.typie.ui.component.entity_container.EntityContainerTopBarTrailing
import co.typie.ui.component.entity_container.EntityContainerTopBarTrailingKey
import co.typie.ui.component.entity_container.calculateEntityReorderOrdersFromOrderedKeys
import co.typie.ui.component.entity_container.displayOrderedEntityItems
import co.typie.ui.component.formatSpaceSummary
import co.typie.ui.component.reorder.rememberReorderableListState
import co.typie.ui.component.reorder.reorderableListContainer
import co.typie.ui.component.sheet.LocalSheetHost
import co.typie.ui.component.topbar.ProvideTopBar
import co.typie.ui.component.topbar.TopBarDefaults
import co.typie.ui.component.topbar.topBarScrollOffset
import co.typie.ui.state.rememberScrollState
import co.typie.ui.theme.AppTheme
import kotlinx.coroutines.launch

private val SpaceScreenPasteBarBottomSpacerHeight = 176.dp
private val SpaceScreenPasteBarBottomOffset = 28.dp

@Composable
fun SpaceScreen() {
  val nav = Nav.current
  val haptic = LocalHapticFeedback.current
  val uriHandler = LocalUriHandler.current
  val sheetHost = LocalSheetHost.current
  val toast = LocalToast.current
  val clipboard = EntityClipboardService
  val model = viewModel { SpaceViewModel() }
  val folderActionModel = viewModel(key = "space-folder-actions") { FolderViewModel() }
  val documentActionModel = viewModel(key = "space-document-actions") { DocumentViewModel() }
  val scrollState = rememberScrollState("space")
  val presenterScope = rememberCoroutineScope()
  var isReordering by remember { mutableStateOf(false) }
  var isPersistingReorder by remember { mutableStateOf(false) }
  var isPasting by remember { mutableStateOf(false) }
  var animatedPasteBarVisible by remember { mutableStateOf(false) }
  var deleteRequest by remember { mutableStateOf<FolderDeleteRequest?>(null) }
  var documentDeleteRequest by remember { mutableStateOf<DocumentDeleteRequest?>(null) }
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
  val reservedBottomSpacerHeight =
    if (animatedPasteBarVisible) {
      SpaceScreenPasteBarBottomSpacerHeight
    } else {
      140.dp
    }

  LaunchedEffect(shouldShowPasteBar) { animatedPasteBarVisible = shouldShowPasteBar }

  LaunchedEffect(isCurrentRoute, animatedPasteBarVisible) {
    if (!isCurrentRoute) {
      return@LaunchedEffect
    }
    toast.bottomInset =
      if (animatedPasteBarVisible) {
        entityPasteBarToastBottomInset(Route.Space.toastBottomInset)
      } else {
        Route.Space.toastBottomInset
      }
  }

  DisposableEffect(Unit) { onDispose { toast.bottomInset = Route.Space.toastBottomInset } }

  LaunchedEffect(site?.id) {
    isReordering = false
    isPersistingReorder = false
  }

  val serverEntities =
    remember(site?.entities) {
      normalizeSpaceEntities(siteName = site?.name.orEmpty(), entities = site?.entities.orEmpty())
    }
  val serverEntityIds = remember(serverEntities) { serverEntities.map { it.id } }
  val editActions =
    listOf(
      EntityContainerEditAction(icon = Lucide.SquareCheck, label = "여러 항목 선택하기"),
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
    trailing =
      if (serverEntities.isEmpty()) null
      else {
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

  ProvideBottomBar(pill = { MainBottomBarPill() }, action = { SpaceBottomBarActionButton() })

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
          folderActionModel.deleteFolderEntity(request.entityId).withDefaultExceptionHandler(toast)
        }
      },
      onDismiss = { deleteRequest = null },
    )
  }

  documentDeleteRequest?.let { request ->
    ConfirmModal(
      title = "문서 삭제",
      message = "\"${request.documentTitle}\" 문서를 삭제하시겠어요? 삭제 후 30일 동안 휴지통에 보관돼요.",
      confirmText = "삭제하기",
      confirmIsDestructive = true,
      onConfirm = {
        documentDeleteRequest = null
        presenterScope.launch {
          documentActionModel.deleteDocument(request.documentId).withDefaultExceptionHandler(toast)
        }
      },
      onDismiss = { documentDeleteRequest = null },
    )
  }

  Screen(
    loading = model.query.state !is QueryState.Success,
    background = AppTheme.colors.surfaceBase,
    contentPadding = PaddingValues(0.dp),
    primaryScrollableState = scrollState,
    body = { contentPadding ->
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
            sheetHost.show(
              documentItemActionsSheet(item) { action ->
                when (action) {
                  FolderAction.Rename -> {
                    sheetHost.show(
                      documentRenameSheet(
                        model = documentActionModel,
                        documentId = item.documentId,
                        initialTitle = item.title,
                      )
                    )
                  }

                  FolderAction.ChangeIcon -> {
                    sheetHost.show(
                      entityIconPickerSheet(
                        model = documentActionModel,
                        entityId = item.id,
                        initialIcon = item.iconName,
                        initialColor = item.iconColor,
                        defaultIconName = "file",
                      )
                    )
                  }

                  FolderAction.OpenExternal -> uriHandler.openUri(item.url)

                  FolderAction.Share -> Unit

                  FolderAction.Move -> {
                    sheetHost.show(entityMoveSheet(source = item.toDocumentTransferSource()))
                  }

                  FolderAction.Copy -> {
                    clipboard.setCopy(
                      sourceSiteId = Preference.siteId!!,
                      items = listOf(item.toDocumentTransferSource()),
                    )
                  }

                  FolderAction.Cut -> {
                    clipboard.setCut(
                      sourceSiteId = Preference.siteId!!,
                      items = listOf(item.toDocumentTransferSource()),
                    )
                  }

                  FolderAction.Delete -> {
                    documentDeleteRequest =
                      DocumentDeleteRequest(
                        documentId = item.documentId,
                        documentTitle = item.title,
                      )
                  }

                  FolderAction.SelectMultiple,
                  FolderAction.StartReorder -> Unit
                }
              }
            )
          },
          onFolderClick = { entityId -> nav.navigate(Route.Folder(entityId)) },
          onFolderLongPress = { item ->
            sheetHost.show(
              folderItemActionsSheet(item) { action ->
                when (action) {
                  FolderAction.Rename -> {
                    sheetHost.show(
                      folderRenameSheet(
                        model = folderActionModel,
                        folderId = item.folderId,
                        initialName = item.name,
                      )
                    )
                  }

                  FolderAction.ChangeIcon -> {
                    sheetHost.show(
                      entityIconPickerSheet(
                        model = folderActionModel,
                        entityId = item.id,
                        initialIcon = item.iconName,
                        initialColor = item.iconColor,
                        defaultIconName = "folder",
                      )
                    )
                  }

                  FolderAction.OpenExternal -> uriHandler.openUri(item.url)

                  FolderAction.Share -> {
                    sheetHost.show(
                      folderShareSheet(
                        model = folderActionModel,
                        folderId = item.folderId,
                        folderUrl = item.url,
                        initialVisibility = requireNotNull(item.visibility),
                        initialThumbnailUrl = item.thumbnailUrl,
                      )
                    )
                  }

                  FolderAction.Move -> {
                    sheetHost.show(entityMoveSheet(source = item.toTransferSource()))
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
                    deleteRequest =
                      FolderDeleteRequest(
                        entityId = item.id,
                        folderName = item.name,
                        shouldPopOnSuccess = false,
                      )
                  }

                  FolderAction.SelectMultiple,
                  FolderAction.StartReorder -> Unit
                }
              }
            )
          },
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

        pasteTarget?.let { resolvedPasteTarget ->
          Box(modifier = Modifier.align(Alignment.BottomCenter)) {
            AnimatedVisibility(
              visible = animatedPasteBarVisible,
              enter =
                fadeIn(animationSpec = tween(220)) +
                  slideInVertically(animationSpec = tween(220), initialOffsetY = { it / 2 }),
              exit =
                fadeOut(animationSpec = tween(180)) +
                  slideOutVertically(animationSpec = tween(180), targetOffsetY = { it / 2 }),
            ) {
              EntityPasteBar(
                bottomOffset = Route.Space.toastBottomInset + SpaceScreenPasteBarBottomOffset,
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
                            toast.show(
                              ToastType.Loading,
                              "${count}개의 항목을 붙여넣는 중이에요",
                              kotlin.time.Duration.ZERO,
                            )
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
          }
        }
      }
    },
  )
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
