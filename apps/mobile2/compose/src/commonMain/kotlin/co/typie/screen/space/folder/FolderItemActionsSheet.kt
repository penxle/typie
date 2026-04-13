package co.typie.screen.space.folder

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.runtime.Composable
import androidx.compose.ui.Modifier
import androidx.compose.ui.unit.dp
import co.typie.entity_transfer.EntityTransferSource
import co.typie.icons.Lucide
import co.typie.ui.component.Divider
import co.typie.ui.component.EntityBreadcrumb
import co.typie.ui.component.EntityHeader
import co.typie.ui.component.EntityListItem
import co.typie.ui.component.EntitySupportingText
import co.typie.ui.component.breadcrumbNames
import co.typie.ui.component.sheet.SheetLayout
import co.typie.ui.component.sheet.SheetMenuActionRow
import co.typie.ui.component.sheet.SheetPadding
import co.typie.ui.component.sheet.SheetScope
import co.typie.ui.component.sheet.dismiss
import co.typie.ui.resolveEntityIconAppearance
import co.typie.ui.theme.AppTheme

private val MenuSheetHorizontalPadding = 24.dp
private val MenuSheetActionContentPadding =
  PaddingValues(horizontal = MenuSheetHorizontalPadding, vertical = 8.dp)
private val MenuSheetRowPadding = PaddingValues(vertical = 12.dp)

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
context(_: SheetScope<Unit>)
internal fun FolderItemActionsContent(
  item: EntityListItem.Folder,
  onAction: (FolderAction) -> Unit,
) {
  val entityIcon =
    resolveEntityIconAppearance(
      iconName = item.iconName,
      iconColor = item.iconColor,
      fallbackIcon = Lucide.Folder,
      fallbackTint = AppTheme.colors.brand,
      colors = AppTheme.colors,
    )
  val visibility =
    folderVisibilityPresentation(visibility = item.visibility, availability = item.availability)

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
          title = item.name,
          icon = entityIcon.icon,
          modifier = Modifier.padding(horizontal = MenuSheetHorizontalPadding),
          iconTint = entityIcon.tint,
        ) {
          EntityBreadcrumb(segments = item.breadcrumbNames())
          EntitySupportingText(
            text = visibility.label,
            color = if (visibility.isShared) AppTheme.colors.brand else AppTheme.colors.textMuted,
          )
          EntitySupportingText(
            text =
              co.typie.ui.component.formatFolderMetadataSummary(
                folderCount = item.folderCount,
                documentCount = item.documentCount,
                characterCount = item.characterCount,
              )
          )
        }

        Divider(color = AppTheme.colors.borderDefault)
      }
    },
  ) {
    Column(modifier = Modifier.fillMaxWidth().padding(MenuSheetActionContentPadding)) {
      folderItemActionSections().forEachIndexed { index, section ->
        if (index > 0) {
          Divider()
        }

        section.items.forEach { action ->
          SheetMenuActionRow(
            icon = action.icon,
            label = action.label,
            contentPadding = MenuSheetRowPadding,
            tint = if (action.isDanger) AppTheme.colors.danger else null,
            trailingIcon = action.trailingIcon,
            onClick = {
              dismiss()
              onAction(action.action)
            },
          )
        }
      }
    }
  }
}
