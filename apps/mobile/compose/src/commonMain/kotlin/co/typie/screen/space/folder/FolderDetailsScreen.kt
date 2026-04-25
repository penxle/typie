package co.typie.screen.space.folder

import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.layout.widthIn
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.platform.LocalUriHandler
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import androidx.lifecycle.viewmodel.compose.viewModel
import co.typie.domain.entity.EntityBreadcrumb
import co.typie.domain.entity.EntityBreadcrumbLayout
import co.typie.domain.entity.EntityIcon
import co.typie.domain.entity.EntityIconPickerSheet
import co.typie.domain.entity.EntityIconPickerStopPolicy
import co.typie.domain.entity.EntityIconPickerStops
import co.typie.domain.entity.EntityMoveSheet
import co.typie.domain.entity.EntityMoveStops
import co.typie.domain.entity.FolderEntityShareSheet
import co.typie.domain.entity.FolderRenameSheet
import co.typie.domain.entity.entityVisibilityPresentation
import co.typie.domain.entity.folder
import co.typie.domain.entity.formatFolderName
import co.typie.domain.entity.isRowEntity
import co.typie.domain.entitytransfer.EntityClipboardService
import co.typie.domain.entitytransfer.toTransferSource
import co.typie.ext.InteractionScope
import co.typie.ext.clickable
import co.typie.ext.comma
import co.typie.ext.pressScale
import co.typie.ext.verticalScroll
import co.typie.graphql.QueryState
import co.typie.graphql.fragment.EntityRowFolder_folder
import co.typie.icons.Lucide
import co.typie.navigation.Nav
import co.typie.result.isOk
import co.typie.result.withDefaultExceptionHandler
import co.typie.route.Route
import co.typie.storage.Preference
import co.typie.ui.component.CardDivider
import co.typie.ui.component.CardRow
import co.typie.ui.component.ResponsiveContainerDefaults
import co.typie.ui.component.Screen
import co.typie.ui.component.Text
import co.typie.ui.component.bleedPadding
import co.typie.ui.component.dialog.DialogResult
import co.typie.ui.component.dialog.LocalDialog
import co.typie.ui.component.dialog.confirm
import co.typie.ui.component.dialog.error
import co.typie.ui.component.sheet.LocalSheet
import co.typie.ui.component.toast.LocalToast
import co.typie.ui.component.topbar.ProvideTopBar
import co.typie.ui.component.topbar.TopBarBackButton
import co.typie.ui.component.topbar.TopBarDefaults
import co.typie.ui.component.topbar.topBarScrollOffset
import co.typie.ui.icon.Icon
import co.typie.ui.icon.IconData
import co.typie.ui.skeleton.Skeleton
import co.typie.ui.state.rememberScrollState
import co.typie.ui.theme.AppShapes
import co.typie.ui.theme.AppTheme
import kotlinx.coroutines.NonCancellable
import kotlinx.coroutines.withContext

@Composable
fun FolderDetailsScreen(entityId: String) {
  val nav = Nav.current
  val uriHandler = LocalUriHandler.current
  val sheet = LocalSheet.current
  val dialog = LocalDialog.current
  val toast = LocalToast.current
  val model = viewModel { FolderViewModel() }
  val scrollState = rememberScrollState()
  val loading = model.query.state !is QueryState.Success

  LaunchedEffect(entityId) { model.entityId = entityId }

  val entity = model.query.data.entity
  val details = entity.entityDetails_entity
  val row = details.entityRow_entity
  val folder = row.folder
  val folderDetails = details.folder
  val folderTitle = folder?.let { formatFolderName(it.name) } ?: "폴더"
  val visibility = entityVisibilityPresentation(details)

  suspend fun popFolderAndMatchingContentIfPresent(): Boolean {
    val previousRoute = nav.previous
    if (previousRoute !is Route.Folder || previousRoute.entityId != entityId) return false

    val targetRoute = nav.stack.getOrNull(nav.stack.lastIndex - 2)
    if (targetRoute != null) {
      nav.popTo(targetRoute)
    } else {
      nav.pop()
    }
    return true
  }

  suspend fun popAfterDelete() {
    withContext(NonCancellable) {
      if (!popFolderAndMatchingContentIfPresent()) {
        nav.pop()
      }
    }
  }

  LaunchedEffect(loading, folder, nav.isTransitioning) {
    if (!loading && folder == null && !nav.isTransitioning) {
      dialog.error(nav = nav, onRetry = { model.query.refetch() })
    }
  }

  ProvideTopBar(
    leading = { TopBarBackButton(icon = Lucide.X) },
    center = {
      if (row.isRowEntity()) {
        Box(modifier = Modifier.fillMaxWidth(), contentAlignment = Alignment.Center) {
          Skeleton.Passive(enabled = loading) {
            Box(
              modifier =
                Modifier.fillMaxWidth()
                  .widthIn(max = ResponsiveContainerDefaults.MaxWidth)
                  .height(TopBarDefaults.TitleHeight)
                  .padding(horizontal = 12.dp),
              contentAlignment = Alignment.CenterStart,
            ) {
              Row(
                modifier = Modifier.fillMaxWidth(),
                verticalAlignment = Alignment.CenterVertically,
              ) {
                EntityIcon(entity = row.entityIcon_entity, modifier = Modifier.size(21.dp))

                Spacer(Modifier.width(12.dp))

                Column(
                  modifier = Modifier.weight(1f),
                  verticalArrangement = Arrangement.spacedBy(2.dp),
                ) {
                  Text(
                    text = folderTitle,
                    style = AppTheme.typography.title.copy(fontSize = 16.sp),
                    maxLines = 1,
                    overflow = TextOverflow.Ellipsis,
                  )
                  Text(
                    text = folderMetadataLabel(folder, folderDetails?.characterCount ?: 0),
                    style = AppTheme.typography.caption.copy(fontSize = 13.sp),
                    color = AppTheme.colors.textMuted,
                    maxLines = 1,
                    overflow = TextOverflow.Ellipsis,
                  )
                }
              }
            }
          }
        }
      }
    },
    scrollOffset = scrollState.topBarScrollOffset(),
  )

  Screen(loadable = model.query, background = AppTheme.colors.surfaceCanvas) { contentPadding ->
    val resolvedFolder = folder ?: return@Screen
    val characterCount = folderDetails?.characterCount ?: 0

    val renameFolder: suspend () -> Unit = {
      if (!loading) {
        sheet.present {
          FolderRenameSheet(
            model = model,
            folderId = resolvedFolder.id,
            initialName = resolvedFolder.name,
            onUpdated = model::refetch,
          )
        }
      }
    }
    val openIconPicker: suspend () -> Unit = {
      if (!loading) {
        sheet.present(stops = EntityIconPickerStops, stopPolicy = EntityIconPickerStopPolicy) {
          EntityIconPickerSheet(
            model = model,
            entityId = row.id,
            initialIcon = row.entityIcon_entity.icon,
            initialColor = row.entityIcon_entity.iconColor,
            defaultIconName = "folder",
            onUpdated = model::refetch,
          )
        }
      }
    }
    val shareFolder: suspend () -> Unit = {
      if (!loading) {
        sheet.present {
          FolderEntityShareSheet(entityIds = listOf(row.id), onUpdated = model::refetch)
        }
      }
    }
    val openInSpace: suspend () -> Unit = {
      if (!loading) {
        row.url.takeIf(String::isNotBlank)?.let(uriHandler::openUri)
      }
    }
    val moveFolder: suspend () -> Unit = {
      if (!loading) {
        sheet.present(stops = EntityMoveStops) {
          EntityMoveSheet(
            source = details.toTransferSource(),
            initialDestinationId = details.ancestors.lastOrNull()?.id,
            onMoved = model::refetch,
          )
        }
      }
    }
    val copyFolderToClipboard: suspend () -> Unit = {
      if (!loading) {
        val sourceSiteId = entity.site.id.takeIf(String::isNotBlank) ?: Preference.siteId
        if (sourceSiteId != null) {
          EntityClipboardService.setCopy(
            sourceSiteId = sourceSiteId,
            items = listOf(details.toTransferSource()),
          )
          toast.success("폴더를 복사했어요.\n원하는 폴더에 붙여넣을 수 있어요.")
        }
      }
    }
    val cutFolderToClipboard: suspend () -> Unit = {
      if (!loading) {
        val sourceSiteId = entity.site.id.takeIf(String::isNotBlank) ?: Preference.siteId
        if (sourceSiteId != null) {
          EntityClipboardService.setCut(
            sourceSiteId = sourceSiteId,
            items = listOf(details.toTransferSource()),
          )
          toast.success("폴더를 잘라냈어요.\n원하는 폴더에 붙여넣을 수 있어요.")
        }
      }
    }
    val deleteFolder: suspend () -> Unit = {
      if (!loading) {
        val result =
          dialog.confirm(
            title = "폴더 삭제",
            message =
              "\"${formatFolderName(resolvedFolder.name)}\" 폴더를 삭제하시겠어요? 삭제 후 30일 동안 휴지통에 보관돼요.",
            confirmText = "삭제하기",
            confirmIsDestructive = true,
          )
        if (result is DialogResult.Resolved) {
          val deleteResult = model.deleteFolderEntity(row.id).withDefaultExceptionHandler(toast)
          if (deleteResult.isOk) {
            popAfterDelete()
          }
        }
      }
    }

    Column(
      modifier =
        Modifier.fillMaxSize()
          .verticalScroll(scrollState)
          .padding(contentPadding)
          .padding(horizontal = 16.dp)
          .padding(bottom = 12.dp)
    ) {
      InteractionScope {
        Box(
          modifier =
            Modifier.size(42.dp)
              .clip(AppShapes.rounded(AppShapes.md))
              .background(AppTheme.colors.surfaceDefault)
              .clickable(onClick = openIconPicker)
              .pressScale(),
          contentAlignment = Alignment.Center,
        ) {
          EntityIcon(entity = row.entityIcon_entity, modifier = Modifier.size(20.dp))
        }
      }

      Spacer(Modifier.height(12.dp))

      Text(text = folderTitle, style = AppTheme.typography.title)

      Spacer(Modifier.height(12.dp))

      EntityBreadcrumb(
        entity = details.entityBreadcrumb_entity,
        layout = EntityBreadcrumbLayout.FlowWrap,
        color = AppTheme.colors.textHint,
      )

      Spacer(Modifier.height(28.dp))

      FolderInfoRow(label = "하위 폴더", value = "${resolvedFolder.folderCount.comma}개")
      Spacer(Modifier.height(8.dp))
      FolderInfoRow(label = "문서", value = "${resolvedFolder.documentCount.comma}개")
      Spacer(Modifier.height(8.dp))
      FolderInfoRow(label = "글자 수", value = "${characterCount.comma}자")

      Box(
        modifier =
          Modifier.fillMaxWidth()
            .padding(top = 18.dp)
            .bleedPadding(PaddingValues(horizontal = 32.dp))
            .height(12.dp)
            .background(AppTheme.colors.surfaceInset)
      )

      FolderActionRow(icon = Lucide.PenLine, label = "이름 변경", onClick = renameFolder)
      FolderActionRow(icon = Lucide.Palette, label = "아이콘 변경", onClick = openIconPicker)

      CardDivider(inset = 0.dp, color = AppTheme.colors.borderDefault)

      FolderActionRow(
        icon = Lucide.Blend,
        label = "공유 및 게시",
        supporting = visibility.label,
        onClick = shareFolder,
      )
      FolderActionRow(
        icon = Lucide.Globe,
        label = "스페이스에서 열기",
        trailingIcon = Lucide.ExternalLink,
        onClick = openInSpace,
      )

      CardDivider(inset = 0.dp, color = AppTheme.colors.borderDefault)

      FolderActionRow(icon = Lucide.FolderSymlink, label = "다른 폴더로 옮기기", onClick = moveFolder)

      CardDivider(inset = 0.dp, color = AppTheme.colors.borderDefault)

      FolderActionRow(icon = Lucide.ClipboardCopy, label = "복사", onClick = copyFolderToClipboard)
      FolderActionRow(icon = Lucide.Scissors, label = "잘라내기", onClick = cutFolderToClipboard)

      CardDivider(inset = 0.dp, color = AppTheme.colors.borderDefault)

      FolderActionRow(
        icon = Lucide.Trash2,
        label = "삭제하기",
        color = AppTheme.colors.danger,
        onClick = deleteFolder,
      )
    }
  }
}

@Composable
private fun FolderInfoRow(
  label: String,
  value: String,
  modifier: Modifier = Modifier,
  valueColor: Color = AppTheme.colors.textMuted.copy(alpha = 0.9f),
) {
  Row(
    modifier = modifier.fillMaxWidth(),
    horizontalArrangement = Arrangement.spacedBy(16.dp),
    verticalAlignment = Alignment.CenterVertically,
  ) {
    Text(
      text = label,
      modifier = Modifier.weight(1f),
      style = AppTheme.typography.caption,
      color = AppTheme.colors.textHint,
    )

    Text(
      text = value,
      style = AppTheme.typography.action.copy(fontWeight = FontWeight.W500),
      color = valueColor,
      maxLines = 2,
      overflow = TextOverflow.Ellipsis,
    )
  }
}

@Composable
private fun FolderActionRow(
  icon: IconData,
  label: String,
  onClick: suspend () -> Unit,
  modifier: Modifier = Modifier,
  supporting: String? = null,
  trailingIcon: IconData? = Lucide.ChevronRight,
  color: Color = AppTheme.colors.textDefault,
) {
  CardRow(
    onClick = onClick,
    modifier = modifier,
    contentPadding = PaddingValues(vertical = 14.dp),
    spacing = 12.dp,
  ) {
    Icon(icon = icon, modifier = Modifier.size(18.dp), tint = color)

    Column(modifier = Modifier.weight(1f)) {
      Text(text = label, style = AppTheme.typography.action, color = color)

      if (supporting != null) {
        Spacer(Modifier.height(4.dp))
        Text(
          text = supporting,
          style = AppTheme.typography.caption,
          color = AppTheme.colors.textMuted,
          maxLines = 2,
          overflow = TextOverflow.Ellipsis,
        )
      }
    }

    if (trailingIcon != null) {
      Icon(icon = trailingIcon, modifier = Modifier.size(15.dp), tint = AppTheme.colors.textHint)
    }
  }
}

private fun folderMetadataLabel(folder: EntityRowFolder_folder?, characterCount: Int): String {
  return listOf(
      folder?.folderCount?.let { "폴더 ${it.comma}개" },
      folder?.documentCount?.let { "문서 ${it.comma}개" },
      "${characterCount.comma}자",
    )
    .filterNotNull()
    .joinToString(" · ")
}
