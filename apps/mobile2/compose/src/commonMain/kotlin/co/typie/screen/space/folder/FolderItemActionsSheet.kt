package co.typie.screen.space.folder

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.ui.Modifier
import androidx.compose.ui.unit.dp
import co.typie.entity_transfer.EntityTransferSource
import co.typie.icons.Lucide
import co.typie.ui.component.EntityListItem
import co.typie.ui.component.breadcrumbNames
import co.typie.ui.component.sheet.SheetDismissReason
import co.typie.ui.component.sheet.SheetEntityBreadcrumb
import co.typie.ui.component.sheet.SheetEntityHeader
import co.typie.ui.component.sheet.SheetEntitySupportingText
import co.typie.ui.component.sheet.SheetLayout
import co.typie.ui.component.sheet.SheetMenuActionRow
import co.typie.ui.component.sheet.SheetMenuDivider
import co.typie.ui.component.sheet.SheetPadding
import co.typie.ui.component.sheet.SheetPresentation
import co.typie.ui.component.sheet.sheetPresentation
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

internal fun folderItemActionsSheet(
  item: EntityListItem.Folder,
  onAction: (FolderAction) -> Unit,
): SheetPresentation<Unit> = sheetPresentation {
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
        SheetEntityHeader(
          title = item.name,
          icon = entityIcon.icon,
          modifier = Modifier.padding(horizontal = MenuSheetHorizontalPadding),
          iconTint = entityIcon.tint,
        ) {
          SheetEntityBreadcrumb(segments = item.breadcrumbNames())
          SheetEntitySupportingText(
            text = visibility.label,
            color = if (visibility.isShared) AppTheme.colors.brand else AppTheme.colors.textMuted,
          )
          SheetEntitySupportingText(
            text =
              co.typie.ui.component.formatFolderMetadataSummary(
                folderCount = item.folderCount,
                documentCount = item.documentCount,
                characterCount = item.characterCount,
              )
          )
        }

        SheetMenuDivider()
      }
    },
  ) {
    Column(modifier = Modifier.fillMaxWidth().padding(MenuSheetActionContentPadding)) {
      folderPrimaryActionSections().forEachIndexed { index, section ->
        if (index > 0) {
          SheetMenuDivider()
        }

        section.items.forEach { action ->
          SheetMenuActionRow(
            icon = action.icon,
            label = action.label,
            contentPadding = MenuSheetRowPadding,
            tint = if (action.isDanger) AppTheme.colors.danger else null,
            trailingIcon = action.trailingIcon,
            onClick = {
              dismiss(SheetDismissReason.Programmatic)
              onAction(action.action)
            },
          )
        }
      }
    }
  }
}
