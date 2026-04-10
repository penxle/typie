package co.typie.screen.space.trash

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.unit.dp
import androidx.lifecycle.viewmodel.compose.viewModel
import co.typie.graphql.QueryState
import co.typie.graphql.TrashScreen_WithEntityId_Query
import co.typie.graphql.TrashScreen_WithSiteId_Query
import co.typie.graphql.type.EntityState
import co.typie.icons.Lucide
import co.typie.navigation.Nav
import co.typie.overlay.LocalToast
import co.typie.overlay.ToastType
import co.typie.result.onOk
import co.typie.result.withDefaultExceptionHandler
import co.typie.route.Route
import co.typie.ui.component.CardDivider
import co.typie.ui.component.CardSurface
import co.typie.ui.component.ConfirmModal
import co.typie.ui.component.ErrorDialog
import co.typie.ui.component.Screen
import co.typie.ui.component.SectionTitle
import co.typie.ui.component.Text
import co.typie.ui.component.TrashDocumentRow
import co.typie.ui.component.TrashFolderRow
import co.typie.ui.component.popover.Popover
import co.typie.ui.component.popover.PopoverDefaults
import co.typie.ui.component.popover.PopoverList
import co.typie.ui.component.popover.PopoverListItem
import co.typie.ui.component.popover.PopoverPlacement
import co.typie.ui.component.popover.PopoverScope
import co.typie.ui.component.sheet.LocalSheetHost
import co.typie.ui.component.sheet.SheetDismissReason
import co.typie.ui.component.sheet.SheetEntityBreadcrumb
import co.typie.ui.component.sheet.SheetEntityHeader
import co.typie.ui.component.sheet.SheetEntitySupportingText
import co.typie.ui.component.sheet.SheetLayout
import co.typie.ui.component.sheet.SheetMenuActionRow
import co.typie.ui.component.sheet.SheetMenuDivider
import co.typie.ui.component.sheet.SheetPadding
import co.typie.ui.component.sheet.SheetPresentation
import co.typie.ui.component.sheet.SheetScope
import co.typie.ui.component.sheet.sheetPresentation
import co.typie.ui.component.topbar.ProvideTopBar
import co.typie.ui.component.topbar.TopBarBackButton
import co.typie.ui.component.topbar.TopBarButton
import co.typie.ui.component.topbar.topBarScrollOffset
import co.typie.ui.icon.Icon
import co.typie.ui.icon.IconData
import co.typie.ui.resolveEntityIconAppearance
import co.typie.ui.state.rememberScrollState
import co.typie.ui.theme.AppTheme
import kotlin.time.Instant
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.launch

private val MenuSheetHorizontalPadding = 24.dp
private val MenuSheetActionContentPadding =
  PaddingValues(horizontal = MenuSheetHorizontalPadding, vertical = 8.dp)
private val MenuSheetRowPadding = PaddingValues(vertical = 12.dp)

internal enum class TrashItemType(val label: String) {
  Folder("폴더"),
  Document("문서"),
}

internal data class TrashItem(
  val id: String,
  val title: String,
  val type: TrashItemType,
  val iconName: String,
  val iconColor: String,
  val subtitle: String? = null,
  val excerpt: String? = null,
  val updatedAt: Instant? = null,
  val siteName: String,
  val ancestorFolderNames: List<String>,
) {
  val breadcrumbSegments: List<String>
    get() =
      buildList {
          add(siteName)
          addAll(ancestorFolderNames)
        }
        .filter { it.isNotBlank() }

  val breadcrumb: String
    get() = breadcrumbSegments.joinToString(" › ")
}

private data class TrashContent(
  val title: String,
  val subtitle: String,
  val currentItem: TrashItem?,
  val items: List<TrashItem>,
  val isRootTrash: Boolean,
) {
  val emptyMessage: String
    get() = if (isRootTrash) "휴지통이 비어있어요" else "폴더가 비어있어요"

  val clearActionLabel: String
    get() = if (isRootTrash) "휴지통 비우기" else "폴더 비우기"
}

private data class TrashPurgeRequest(
  val title: String,
  val message: String,
  val confirmText: String,
  val entityIds: List<String>,
  val successMessage: String,
  val shouldPop: Boolean = false,
)

private data class TrashActionItem(
  val label: String,
  val icon: IconData,
  val tint: Color? = null,
  val onClick: suspend () -> Unit,
)

@Composable
fun TrashScreen(entityId: String? = null) {
  val nav = Nav.current
  val toast = LocalToast.current
  val model = viewModel(key = "trash:${entityId ?: "site"}") { TrashViewModel() }
  val sheetHost = LocalSheetHost.current
  val screenScope = rememberCoroutineScope()
  val scrollState = rememberScrollState()
  var purgeRequest by remember(entityId) { mutableStateOf<TrashPurgeRequest?>(null) }

  LaunchedEffect(entityId) {
    model.entityId = entityId
    model.refetch()
  }

  val queryState = if (entityId == null) model.siteQuery.state else model.entityQuery.state
  val content = trashContent(queryState = queryState)
  val dangerColor = AppTheme.colors.danger

  fun showItemActionsSheet(item: TrashItem) {
    sheetHost.show(
      sheet =
        trashActionsSheet(
          item = item,
          actions =
            listOf(
              TrashActionItem(
                label = "복원",
                icon = Lucide.Undo2,
                onClick = {
                  model.recoverEntity(item).withDefaultExceptionHandler(toast).onOk { message ->
                    toast.show(ToastType.Success, message)
                    model.refetch()
                  }
                },
              ),
              TrashActionItem(
                label = "영구 삭제",
                icon = Lucide.Trash2,
                tint = dangerColor,
                onClick = {
                  purgeRequest =
                    TrashPurgeRequest(
                      title = "${item.type.label} 영구 삭제",
                      message = "영구 삭제한 ${item.type.label}는 복원할 수 없어요. 정말 삭제하시겠어요?",
                      confirmText = "삭제",
                      entityIds = listOf(item.id),
                      successMessage = "\"${item.title}\" ${item.type.label}가 영구 삭제되었어요.",
                    )
                },
              ),
            ),
          onAction = { action -> sheetHost.launch { action.onClick() } },
        )
    )
  }

  LaunchedEffect(queryState, entityId) {
    val data =
      (queryState as? QueryState.Success<*>)?.data as? TrashScreen_WithEntityId_Query.Data
        ?: return@LaunchedEffect
    if (entityId != null && data.entity.state != EntityState.DELETED) {
      nav.pop()
    }
  }

  val topBarActions =
    if (queryState is QueryState.Success) {
      buildList {
        val currentItem = content.currentItem
        if (currentItem != null) {
          add(
            TrashActionItem(
              label = "복원",
              icon = Lucide.Undo2,
              onClick = {
                model.recoverEntity(currentItem).withDefaultExceptionHandler(toast).onOk { message
                  ->
                  toast.show(ToastType.Success, message)
                  nav.pop()
                }
              },
            )
          )
          add(
            TrashActionItem(
              label = "영구 삭제",
              icon = Lucide.Trash2,
              tint = AppTheme.colors.danger,
              onClick = {
                purgeRequest =
                  TrashPurgeRequest(
                    title = "${currentItem.type.label} 영구 삭제",
                    message = "영구 삭제한 ${currentItem.type.label}는 복원할 수 없어요. 정말 삭제하시겠어요?",
                    confirmText = "삭제",
                    entityIds = listOf(currentItem.id),
                    successMessage =
                      "\"${currentItem.title}\" ${currentItem.type.label}가 영구 삭제되었어요.",
                    shouldPop = true,
                  )
              },
            )
          )
        }

        add(
          TrashActionItem(
            label = content.clearActionLabel,
            icon = Lucide.BrushCleaning,
            tint = AppTheme.colors.danger,
            onClick = {
              if (content.items.isEmpty()) {
                toast.show(ToastType.Notification, content.emptyMessage)
              } else {
                purgeRequest =
                  TrashPurgeRequest(
                    title = content.clearActionLabel,
                    message =
                      if (content.isRootTrash) {
                        "휴지통에 있는 ${content.items.size}개 항목을 모두 영구 삭제할까요? 삭제된 항목은 복원할 수 없어요."
                      } else {
                        "이 폴더에 있는 ${content.items.size}개 항목을 모두 영구 삭제할까요? 삭제된 항목은 복원할 수 없어요."
                      },
                    confirmText = "비우기",
                    entityIds = content.items.map { it.id },
                    successMessage = if (content.isRootTrash) "휴지통을 비웠어요." else "폴더를 비웠어요.",
                  )
              }
            },
          )
        )
      }
    } else {
      emptyList()
    }

  ProvideTopBar(
    leading = { TopBarBackButton() },
    center = { Text(content.title, style = AppTheme.typography.title) },
    trailing = {
      if (topBarActions.isNotEmpty()) {
        TrashTopBarMenu(actions = topBarActions, actionScope = screenScope)
      }
    },
    scrollOffset = scrollState.topBarScrollOffset(),
  )

  if (queryState is QueryState.Error) {
    ErrorDialog { model.refetch() }
  }

  Screen(
    scrollState = scrollState,
    loading = queryState !is QueryState.Success,
    background = AppTheme.colors.surfaceBase,
    extraPadding = PaddingValues(bottom = 16.dp),
    verticalArrangement = Arrangement.spacedBy(16.dp),
  ) {
    Text(
      text = content.title,
      style = AppTheme.typography.display,
      modifier = Modifier.padding(top = 4.dp),
    )

    Text(
      text = content.subtitle,
      style = AppTheme.typography.body,
      color = AppTheme.colors.textTertiary,
    )

    SectionTitle("삭제된 항목")

    if (content.items.isEmpty()) {
      CardSurface(modifier = Modifier.fillMaxWidth()) {
        Box(
          modifier = Modifier.fillMaxWidth().padding(vertical = 36.dp, horizontal = 20.dp),
          contentAlignment = Alignment.Center,
        ) {
          Text(
            text = content.emptyMessage,
            style = AppTheme.typography.label,
            color = AppTheme.colors.textTertiary,
          )
        }
      }
    } else {
      CardSurface(modifier = Modifier.fillMaxWidth()) {
        Column {
          content.items.forEachIndexed { index, item ->
            if (index > 0) {
              CardDivider()
            }

            when (item.type) {
              TrashItemType.Folder -> {
                TrashFolderRow(
                  title = item.title,
                  iconName = item.iconName,
                  iconColor = item.iconColor,
                  onLongPress = { showItemActionsSheet(item) },
                  onClick = { nav.navigate(Route.Trash(item.id)) },
                )
              }

              TrashItemType.Document -> {
                TrashDocumentRow(
                  title = item.title,
                  subtitle = item.subtitle,
                  excerpt = item.excerpt,
                  updatedAt = item.updatedAt,
                  iconName = item.iconName,
                  iconColor = item.iconColor,
                  onLongPress = { showItemActionsSheet(item) },
                  onClick = { showItemActionsSheet(item) },
                )
              }
            }
          }
        }
      }
    }

    Spacer(Modifier.height(72.dp))
  }

  if (purgeRequest != null) {
    ConfirmModal(
      title = purgeRequest!!.title,
      message = purgeRequest!!.message,
      confirmText = purgeRequest!!.confirmText,
      confirmIsDestructive = true,
      onConfirm = {
        val request = purgeRequest
        if (request != null) {
          purgeRequest = null
          screenScope.launch {
            model.purgeEntities(request.entityIds).withDefaultExceptionHandler(toast).onOk {
              toast.show(ToastType.Success, request.successMessage)
              if (request.shouldPop) {
                nav.pop()
              } else {
                model.refetch()
              }
            }
          }
        }
      },
      onDismiss = { purgeRequest = null },
    )
  }
}

private fun trashActionsSheet(
  item: TrashItem,
  actions: List<TrashActionItem>,
  onAction: (TrashActionItem) -> Unit,
): SheetPresentation<Unit> = sheetPresentation {
  TrashActionsSheetContent(item = item, actions = actions, onAction = onAction)
}

@Composable
private fun SheetScope<Unit>.TrashActionsSheetContent(
  item: TrashItem,
  actions: List<TrashActionItem>,
  onAction: (TrashActionItem) -> Unit,
) {
  val entityIcon =
    resolveEntityIconAppearance(
      iconName = item.iconName,
      iconColor = item.iconColor,
      fallbackIcon = if (item.type == TrashItemType.Folder) Lucide.Folder else Lucide.File,
      fallbackTint =
        if (item.type == TrashItemType.Folder) AppTheme.colors.brand
        else AppTheme.colors.textSecondary,
      colors = AppTheme.colors,
    )

  SheetLayout(
    bodyScroll = false,
    padding = SheetPadding.None,
    verticalSpacing = 0.dp,
    header = {
      Column(
        modifier = Modifier.fillMaxWidth(),
        verticalArrangement = Arrangement.spacedBy(16.dp),
      ) {
        SheetEntityHeader(
          title = item.title,
          icon = entityIcon.icon,
          modifier = Modifier.padding(horizontal = MenuSheetHorizontalPadding),
          iconTint = entityIcon.tint,
        ) {
          SheetEntityBreadcrumb(segments = item.breadcrumbSegments)
          SheetEntitySupportingText(text = "삭제됨")
        }

        SheetMenuDivider()
      }
    },
  ) {
    Column(modifier = Modifier.fillMaxWidth().padding(MenuSheetActionContentPadding)) {
      actions.forEach { action ->
        SheetMenuActionRow(
          icon = action.icon,
          label = action.label,
          contentPadding = MenuSheetRowPadding,
          tint = action.tint,
          onClick = {
            dismiss(SheetDismissReason.Programmatic)
            onAction(action)
          },
        )
      }
    }
  }
}

@Composable
private fun TrashTopBarMenu(actions: List<TrashActionItem>, actionScope: CoroutineScope) {
  Popover(
    placement = PopoverPlacement.BelowEnd,
    anchor = { TopBarButton(icon = Lucide.Ellipsis) },
    pane = { TrashTopBarMenuPane(actions = actions, actionScope = actionScope) },
  )
}

@Composable
private fun PopoverScope.TrashTopBarMenuPane(
  actions: List<TrashActionItem>,
  actionScope: CoroutineScope,
) {
  Column(modifier = Modifier.padding(PopoverDefaults.PanePadding)) {
    PopoverList(
      items =
        actions.map { action ->
          PopoverListItem(
            content = {
              TrashActionLabel(
                action = action,
                modifier = Modifier.height(42.dp).padding(horizontal = 16.dp),
              )
            },
            onSelected = {
              close()
              actionScope.launch { action.onClick() }
            },
          )
        }
    )
  }
}

@Composable
private fun TrashActionLabel(action: TrashActionItem, modifier: Modifier = Modifier) {
  Row(
    modifier = modifier,
    verticalAlignment = Alignment.CenterVertically,
    horizontalArrangement = Arrangement.spacedBy(12.dp),
  ) {
    Icon(
      icon = action.icon,
      modifier = Modifier.size(18.dp),
      tint = action.tint ?: AppTheme.colors.textPrimary,
    )
    Text(
      text = action.label,
      style = AppTheme.typography.action,
      color = action.tint ?: AppTheme.colors.textPrimary,
    )
  }
}

private fun trashContent(queryState: QueryState<*>): TrashContent {
  return when (queryState) {
    is QueryState.Success<*> -> {
      when (val data = queryState.data) {
        is TrashScreen_WithSiteId_Query.Data ->
          TrashContent(
            title = "휴지통",
            subtitle = "${data.site.name} 스페이스의 삭제된 항목이에요",
            currentItem = null,
            items = data.site.deletedEntities.map { it.toTrashItem(siteName = data.site.name) },
            isRootTrash = true,
          )

        is TrashScreen_WithEntityId_Query.Data -> {
          val currentItem = data.entity.toTrashItem()
          TrashContent(
            title = currentItem.title,
            subtitle = "이 폴더에서 삭제된 항목이에요",
            currentItem = currentItem,
            items =
              data.entity.deletedChildren.map { it.toTrashItem(siteName = data.entity.site.name) },
            isRootTrash = false,
          )
        }

        else ->
          TrashContent(
            title = "휴지통",
            subtitle = "",
            currentItem = null,
            items = emptyList(),
            isRootTrash = true,
          )
      }
    }

    else ->
      TrashContent(
        title = "휴지통",
        subtitle = "",
        currentItem = null,
        items = emptyList(),
        isRootTrash = true,
      )
  }
}

private fun TrashScreen_WithEntityId_Query.Entity.toTrashItem(): TrashItem {
  return when {
    node.onFolder != null ->
      TrashItem(
        id = id,
        title = node.onFolder.name,
        type = TrashItemType.Folder,
        iconName = icon,
        iconColor = iconColor,
        siteName = site.name,
        ancestorFolderNames = ancestors.mapNotNull { it.node.onFolder?.name },
      )

    node.onDocument != null ->
      TrashItem(
        id = id,
        title = node.onDocument.title,
        type = TrashItemType.Document,
        iconName = icon,
        iconColor = iconColor,
        subtitle = node.onDocument.subtitle,
        excerpt = node.onDocument.excerpt,
        updatedAt = node.onDocument.updatedAt,
        siteName = site.name,
        ancestorFolderNames = ancestors.mapNotNull { it.node.onFolder?.name },
      )

    else ->
      TrashItem(
        id = id,
        title = "삭제된 항목",
        type = TrashItemType.Document,
        iconName = "",
        iconColor = "",
        siteName = site.name,
        ancestorFolderNames = ancestors.mapNotNull { it.node.onFolder?.name },
      )
  }
}

private fun TrashScreen_WithSiteId_Query.DeletedEntity.toTrashItem(siteName: String): TrashItem {
  return when {
    node.onFolder != null ->
      TrashItem(
        id = id,
        title = node.onFolder.name,
        type = TrashItemType.Folder,
        iconName = icon,
        iconColor = iconColor,
        siteName = siteName,
        ancestorFolderNames = ancestors.mapNotNull { it.node.onFolder?.name },
      )

    node.onDocument != null ->
      TrashItem(
        id = id,
        title = node.onDocument.title,
        type = TrashItemType.Document,
        iconName = icon,
        iconColor = iconColor,
        subtitle = node.onDocument.subtitle,
        excerpt = node.onDocument.excerpt,
        updatedAt = node.onDocument.updatedAt,
        siteName = siteName,
        ancestorFolderNames = ancestors.mapNotNull { it.node.onFolder?.name },
      )

    else ->
      TrashItem(
        id = id,
        title = "삭제된 항목",
        type = TrashItemType.Document,
        iconName = "",
        iconColor = "",
        siteName = siteName,
        ancestorFolderNames = ancestors.mapNotNull { it.node.onFolder?.name },
      )
  }
}

private fun TrashScreen_WithEntityId_Query.DeletedChildren.toTrashItem(
  siteName: String
): TrashItem {
  return when {
    node.onFolder != null ->
      TrashItem(
        id = id,
        title = node.onFolder.name,
        type = TrashItemType.Folder,
        iconName = icon,
        iconColor = iconColor,
        siteName = siteName,
        ancestorFolderNames = ancestors.mapNotNull { it.node.onFolder?.name },
      )

    node.onDocument != null ->
      TrashItem(
        id = id,
        title = node.onDocument.title,
        type = TrashItemType.Document,
        iconName = icon,
        iconColor = iconColor,
        subtitle = node.onDocument.subtitle,
        excerpt = node.onDocument.excerpt,
        updatedAt = node.onDocument.updatedAt,
        siteName = siteName,
        ancestorFolderNames = ancestors.mapNotNull { it.node.onFolder?.name },
      )

    else ->
      TrashItem(
        id = id,
        title = "삭제된 항목",
        type = TrashItemType.Document,
        iconName = "",
        iconColor = "",
        siteName = siteName,
        ancestorFolderNames = ancestors.mapNotNull { it.node.onFolder?.name },
      )
  }
}
