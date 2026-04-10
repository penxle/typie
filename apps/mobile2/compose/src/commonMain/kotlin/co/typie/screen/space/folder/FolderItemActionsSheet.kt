package co.typie.screen.space.folder

import androidx.compose.runtime.Composable
import androidx.compose.ui.Modifier
import co.typie.entity_transfer.EntityTransferSource
import co.typie.icons.Lucide
import co.typie.ui.resolveEntityIconAppearance
import co.typie.ui.component.EntityListItem
import co.typie.ui.component.breadcrumbNames
import co.typie.ui.component.bottomsheet.BottomSheetEntityBreadcrumb
import co.typie.ui.component.bottomsheet.BottomSheetEntityHeader
import co.typie.ui.component.bottomsheet.BottomSheetEntitySupportingText
import co.typie.ui.component.bottomsheet.BottomSheetMenu
import co.typie.ui.component.bottomsheet.BottomSheetMenuActionRow
import co.typie.ui.component.bottomsheet.BottomSheetMenuDivider
import co.typie.ui.component.bottomsheet.BottomSheetScope
import co.typie.ui.component.bottomsheet.dismiss
import co.typie.ui.theme.AppTheme
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.launch

internal data class FolderDeleteRequest(
  val entityId: String,
  val folderName: String,
  val shouldPopOnSuccess: Boolean,
)

internal fun EntityListItem.Folder.toTransferSource(): EntityTransferSource.Folder {
  return EntityTransferSource.Folder(
    id = id,
    title = name,
    depth = depth,
    maxDescendantFoldersDepth = maxDescendantFoldersDepth,
  )
}

@Composable
internal fun BottomSheetScope<Unit>.FolderItemActionsSheet(
  item: EntityListItem.Folder,
  actionScope: CoroutineScope,
  onAction: suspend (FolderAction) -> Unit,
) {
  val entityIcon = resolveEntityIconAppearance(
    iconName = item.iconName,
    iconColor = item.iconColor,
    fallbackIcon = Lucide.Folder,
    fallbackTint = AppTheme.colors.brand,
    colors = AppTheme.colors,
  )
  val visibility = folderVisibilityPresentation(
    visibility = item.visibility,
    availability = item.availability,
  )

  BottomSheetMenu(
    showHeaderDivider = false,
    header = {
      BottomSheetEntityHeader(
        title = item.name,
        icon = entityIcon.icon,
        iconTint = entityIcon.tint,
      ) {
        BottomSheetEntityBreadcrumb(segments = item.breadcrumbNames())
        BottomSheetEntitySupportingText(
          text = visibility.label,
          color = if (visibility.isShared) AppTheme.colors.brand else AppTheme.colors.textMuted,
        )
        BottomSheetEntitySupportingText(
          text = co.typie.ui.component.formatFolderMetadataSummary(
            folderCount = item.folderCount,
            documentCount = item.documentCount,
            characterCount = item.characterCount,
          ),
        )
      }
    },
  ) {
    folderPrimaryActionSections().forEachIndexed { index, section ->
      if (index > 0) {
        BottomSheetMenuDivider()
      }

      section.items.forEach { action ->
        BottomSheetMenuActionRow(
          icon = action.icon,
          label = action.label,
          tint = if (action.isDanger) AppTheme.colors.danger else null,
          trailingIcon = action.trailingIcon,
          onClick = {
            dismiss()
            actionScope.launch {
              onAction(action.action)
            }
          },
        )
      }
    }
  }
}
