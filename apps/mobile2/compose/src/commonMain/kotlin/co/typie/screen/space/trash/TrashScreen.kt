package co.typie.screen.space.trash

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.unit.dp
import androidx.lifecycle.ViewModel
import androidx.lifecycle.viewModelScope
import androidx.lifecycle.viewmodel.compose.viewModel
import co.typie.datetime.timeAgo
import co.typie.domain.entity.EntityHeader
import co.typie.domain.entity.EntityRow
import co.typie.domain.entity.EntityRowChevron
import co.typie.domain.entity.displayTitle
import co.typie.domain.entity.document
import co.typie.domain.entity.folder
import co.typie.domain.entity.formatEntityExcerpt
import co.typie.domain.entity.isFolder
import co.typie.ext.verticalScroll
import co.typie.graphql.Apollo
import co.typie.graphql.PlaceholderResolver
import co.typie.graphql.QueryState
import co.typie.graphql.TrashScreen_Folder_Query
import co.typie.graphql.TrashScreen_ItemActions_Query
import co.typie.graphql.TrashScreen_Root_Query
import co.typie.graphql.WatchQuery
import co.typie.graphql.builder.Data
import co.typie.graphql.builder.buildDocument
import co.typie.graphql.builder.buildEntity
import co.typie.graphql.builder.buildFolder
import co.typie.graphql.builder.buildSite
import co.typie.graphql.fragment.EntityRow_entity
import co.typie.graphql.text
import co.typie.graphql.type.EntityState
import co.typie.graphql.watchQuery
import co.typie.icons.Lucide
import co.typie.navigation.Nav
import co.typie.result.onOk
import co.typie.result.withDefaultExceptionHandler
import co.typie.route.Route
import co.typie.ui.component.CardDivider
import co.typie.ui.component.CardSurface
import co.typie.ui.component.Divider
import co.typie.ui.component.Screen
import co.typie.ui.component.SectionTitle
import co.typie.ui.component.Text
import co.typie.ui.component.dialog.DialogResult
import co.typie.ui.component.dialog.LocalDialog
import co.typie.ui.component.dialog.confirm
import co.typie.ui.component.popover.PopoverMenu
import co.typie.ui.component.sheet.LocalSheet
import co.typie.ui.component.sheet.SheetActionRow
import co.typie.ui.component.sheet.SheetLayout
import co.typie.ui.component.sheet.SheetPadding
import co.typie.ui.component.sheet.SheetScope
import co.typie.ui.component.sheet.dismiss
import co.typie.ui.component.toast.LocalToast
import co.typie.ui.component.toast.ToastType
import co.typie.ui.component.topbar.ProvideTopBar
import co.typie.ui.component.topbar.TopBarBackButton
import co.typie.ui.component.topbar.TopBarButton
import co.typie.ui.component.topbar.topBarScrollOffset
import co.typie.ui.icon.IconData
import co.typie.ui.state.rememberScrollState
import co.typie.ui.theme.AppTheme
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.launch

private val MenuSheetHorizontalPadding = 24.dp
private val MenuSheetActionContentPadding =
  PaddingValues(horizontal = MenuSheetHorizontalPadding, vertical = 8.dp)
private val MenuSheetRowPadding = PaddingValues(vertical = 12.dp)

private data class TrashContent(
  val title: String,
  val subtitle: String,
  val currentItem: EntityRow_entity?,
  val items: List<EntityRow_entity>,
  val isRootTrash: Boolean,
) {
  val emptyMessage: String
    get() = if (isRootTrash) "휴지통이 비어있어요" else "폴더가 비어있어요"

  val clearActionLabel: String
    get() = if (isRootTrash) "휴지통 비우기" else "폴더 비우기"
}

private fun trashItemLabel(item: EntityRow_entity): String {
  return if (item.isFolder()) "폴더" else "문서"
}

private data class TrashActionItem(
  val label: String,
  val icon: IconData,
  val tint: Color? = null,
  val onClick: suspend () -> Unit,
)

private class TrashItemActionsViewModel(initialEntity: EntityRow_entity) : ViewModel() {
  val query =
    Apollo.watchQuery(
      scope = viewModelScope,
      placeholderData = trashItemActionsPlaceholderData(initialEntity),
      skip = { initialEntity.id.isBlank() },
    ) {
      TrashScreen_ItemActions_Query(entityId = initialEntity.id)
    }
}

@Composable
fun TrashScreen(entityId: String? = null) {
  val nav = Nav.current
  val dialog = LocalDialog.current
  val toast = LocalToast.current
  val model = viewModel { TrashViewModel() }
  val sheet = LocalSheet.current
  val screenScope = rememberCoroutineScope()
  val scrollState = rememberScrollState()
  LaunchedEffect(entityId) {
    model.entityId = entityId
    model.refetch()
  }

  val queryState = if (entityId == null) model.siteQuery.state else model.entityQuery.state
  val content =
    if (entityId == null) {
      trashContent(model.siteQuery.data)
    } else {
      trashContent(model.entityQuery.data)
    }
  val dangerColor = AppTheme.colors.danger

  fun showItemActionsSheet(item: EntityRow_entity) {
    screenScope.launch {
      sheet.present {
        TrashActionsContent(
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
                  val itemLabel = trashItemLabel(item)
                  val result =
                    dialog.confirm(
                      title = "${itemLabel} 영구 삭제",
                      message = "영구 삭제한 ${itemLabel}는 복원할 수 없어요. 정말 삭제하시겠어요?",
                      confirmText = "삭제",
                      confirmIsDestructive = true,
                    )
                  if (result is DialogResult.Resolved) {
                    model.purgeEntities(listOf(item.id)).withDefaultExceptionHandler(toast).onOk {
                      toast.show(
                        ToastType.Success,
                        "\"${item.displayTitle()}\" ${itemLabel}가 영구 삭제되었어요.",
                      )
                      model.refetch()
                    }
                  }
                },
              ),
            ),
          onAction = { action -> screenScope.launch { action.onClick() } },
        )
      }
    }
  }

  LaunchedEffect(queryState, entityId) {
    val data =
      (queryState as? QueryState.Success<*>)?.data as? TrashScreen_Folder_Query.Data
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
          val currentItemLabel = trashItemLabel(currentItem)
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
                val result =
                  dialog.confirm(
                    title = "${currentItemLabel} 영구 삭제",
                    message = "영구 삭제한 ${currentItemLabel}는 복원할 수 없어요. 정말 삭제하시겠어요?",
                    confirmText = "삭제",
                    confirmIsDestructive = true,
                  )
                if (result is DialogResult.Resolved) {
                  model
                    .purgeEntities(listOf(currentItem.id))
                    .withDefaultExceptionHandler(toast)
                    .onOk {
                      toast.show(
                        ToastType.Success,
                        "\"${currentItem.displayTitle()}\" ${currentItemLabel}가 영구 삭제되었어요.",
                      )
                      nav.pop()
                    }
                }
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
                val result =
                  dialog.confirm(
                    title = content.clearActionLabel,
                    message =
                      if (content.isRootTrash) {
                        "휴지통에 있는 ${content.items.size}개 항목을 모두 영구 삭제할까요? 삭제된 항목은 복원할 수 없어요."
                      } else {
                        "이 폴더에 있는 ${content.items.size}개 항목을 모두 영구 삭제할까요? 삭제된 항목은 복원할 수 없어요."
                      },
                    confirmText = "비우기",
                    confirmIsDestructive = true,
                  )
                if (result is DialogResult.Resolved) {
                  val entityIds = content.items.map { it.id }
                  val successMessage = if (content.isRootTrash) "휴지통을 비웠어요." else "폴더를 비웠어요."
                  model.purgeEntities(entityIds).withDefaultExceptionHandler(toast).onOk {
                    toast.show(ToastType.Success, successMessage)
                    model.refetch()
                  }
                }
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
    center = {
      Text(
        if (queryState is QueryState.Success) content.title else "휴지통",
        style = AppTheme.typography.title,
      )
    },
    trailing = {
      if (topBarActions.isNotEmpty()) {
        TrashTopBarMenu(actions = topBarActions, actionScope = screenScope)
      }
    },
    scrollOffset = scrollState.topBarScrollOffset(),
  )

  Screen(loadable = if (entityId == null) model.siteQuery else model.entityQuery) { contentPadding
    ->
    Column(
      modifier =
        Modifier.fillMaxSize()
          .verticalScroll(scrollState)
          .padding(contentPadding)
          .padding(bottom = 16.dp),
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

              if (item.isFolder()) {
                TrashEntityRow(
                  item = item,
                  onLongPress = { showItemActionsSheet(item) },
                  onClick = { nav.navigate(Route.Trash(item.id)) },
                )
              } else {
                TrashEntityRow(
                  item = item,
                  onLongPress = { showItemActionsSheet(item) },
                  onClick = { showItemActionsSheet(item) },
                )
              }
            }
          }
        }
      }

      Spacer(Modifier.height(72.dp))
    }
  }
}

@Composable
private fun TrashEntityRow(
  item: EntityRow_entity,
  onLongPress: suspend () -> Unit,
  onClick: suspend () -> Unit,
) {
  EntityRow(
    entity = item,
    trailing =
      if (item.isFolder()) {
        { EntityRowChevron() }
      } else {
        null
      },
    onLongPress = onLongPress,
    onClick = onClick,
  ) {
    if (item.isFolder()) {
      title(title = item.displayTitle())
      supporting(text = "삭제된 폴더")
    } else {
      title(
        title = item.displayTitle(),
        subtitle = item.document?.subtitle,
        trailingText = item.document?.updatedAt?.timeAgo(),
      )
      supporting(text = formatEntityExcerpt(item.document?.excerpt.orEmpty()))
    }
  }
}

@Composable
context(_: SheetScope<Unit>)
private fun TrashActionsContent(
  item: EntityRow_entity,
  actions: List<TrashActionItem>,
  onAction: (TrashActionItem) -> Unit,
) {
  val itemActionsQuery = rememberTrashItemActionsQuery(item)
  val resolved = itemActionsQuery.state is QueryState.Success
  val headerEntity = itemActionsQuery.data.entity.trashActionsHeader_entity
  val rowEntity = headerEntity.entityRow_entity

  SheetLayout(
    bodyScroll = false,
    padding = SheetPadding.None,
    verticalSpacing = 0.dp,
    header = {
      Column(
        modifier = Modifier.fillMaxWidth(),
        verticalArrangement = Arrangement.spacedBy(16.dp),
      ) {
        EntityHeader(
          title = rowEntity.displayTitle(),
          entityIcon = rowEntity.entityIcon_entity,
          modifier = Modifier.padding(horizontal = MenuSheetHorizontalPadding),
        ) {
          breadcrumb(entity = headerEntity.entityBreadcrumb_entity, loading = !resolved)
          supportingText("삭제됨")
        }

        Divider(color = AppTheme.colors.borderDefault)
      }
    },
  ) {
    Column(modifier = Modifier.fillMaxWidth().padding(MenuSheetActionContentPadding)) {
      actions.forEach { action ->
        SheetActionRow(
          icon = action.icon,
          label = action.label,
          contentPadding = MenuSheetRowPadding,
          tint = action.tint,
          onClick = {
            dismiss()
            onAction(action)
          },
        )
      }
    }
  }
}

@Composable
private fun TrashTopBarMenu(actions: List<TrashActionItem>, actionScope: CoroutineScope) {
  PopoverMenu(anchor = { TopBarButton(icon = Lucide.Ellipsis) }) {
    actions.forEach { action ->
      item(icon = action.icon, label = action.label, color = action.tint) {
        actionScope.launch { action.onClick() }
      }
    }
  }
}

@Composable
private fun rememberTrashItemActionsQuery(
  initialEntity: EntityRow_entity
): WatchQuery<TrashScreen_ItemActions_Query.Data, TrashScreen_ItemActions_Query.Data> {
  val model = viewModel { TrashItemActionsViewModel(initialEntity) }

  return model.query
}

private fun trashContent(data: TrashScreen_Root_Query.Data): TrashContent =
  TrashContent(
    title = "휴지통",
    subtitle = "${data.site.name} 스페이스의 삭제된 항목이에요",
    currentItem = null,
    items = data.site.deletedEntities.map { it.entityRow_entity },
    isRootTrash = true,
  )

private fun trashContent(data: TrashScreen_Folder_Query.Data): TrashContent {
  val currentItem = data.entity.entityRow_entity
  return TrashContent(
    title = currentItem.displayTitle(),
    subtitle = "이 폴더에서 삭제된 항목이에요",
    currentItem = currentItem,
    items = data.entity.deletedChildren.map { it.entityRow_entity },
    isRootTrash = false,
  )
}

private fun trashItemActionsPlaceholderData(initialEntity: EntityRow_entity) =
  TrashScreen_ItemActions_Query.Data(PlaceholderResolver) {
    entity = buildEntity {
      id = initialEntity.id
      depth = initialEntity.depth
      order = initialEntity.order
      slug = initialEntity.slug
      url = initialEntity.url
      type = initialEntity.entityIcon_entity.type
      icon = initialEntity.entityIcon_entity.icon
      iconColor = initialEntity.entityIcon_entity.iconColor
      site = buildSite {
        id = "placeholder-site"
        name = text(4..8)
      }
      ancestors = emptyList()
      node =
        initialEntity.folder?.let { folder ->
          buildFolder {
            id = folder.id
            name = folder.name
            maxDescendantFoldersDepth = folder.maxDescendantFoldersDepth
            folderCount = folder.folderCount
            documentCount = folder.documentCount
          }
        }
          ?: initialEntity.document?.let { document ->
            buildDocument {
              id = document.id
              title = document.title
              subtitle = document.subtitle
              excerpt = document.excerpt
              updatedAt = document.updatedAt
            }
          }
          ?: error("Unsupported entity type for trash item actions placeholder")
    }
  }
