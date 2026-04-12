package co.typie.screen.space.document

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.ui.Modifier
import androidx.compose.ui.unit.dp
import co.typie.entity_transfer.EntityTransferSource
import co.typie.ext.comma
import co.typie.icons.Lucide
import co.typie.screen.space.folder.FolderAction
import co.typie.screen.space.folder.folderPrimaryActionSections
import co.typie.screen.space.folder.folderVisibilityPresentation
import co.typie.ui.component.Divider
import co.typie.ui.component.EntityBreadcrumb
import co.typie.ui.component.EntityHeader
import co.typie.ui.component.EntityListItem
import co.typie.ui.component.EntitySupportingText
import co.typie.ui.component.breadcrumbNames
import co.typie.ui.component.sheet.SheetDismissReason
import co.typie.ui.component.sheet.SheetLayout
import co.typie.ui.component.sheet.SheetMenuActionRow
import co.typie.ui.component.sheet.SheetPadding
import co.typie.ui.component.sheet.SheetPresentation
import co.typie.ui.component.sheet.sheetPresentation
import co.typie.ui.resolveEntityIconAppearance
import co.typie.ui.theme.AppTheme

private val MenuSheetHorizontalPadding = 24.dp
private val MenuSheetActionContentPadding =
  PaddingValues(horizontal = MenuSheetHorizontalPadding, vertical = 8.dp)
private val MenuSheetRowPadding = PaddingValues(vertical = 12.dp)

internal data class DocumentDeleteRequest(val documentId: String, val documentTitle: String)

internal fun EntityListItem.Document.toTransferSource(): EntityTransferSource.Document {
  return EntityTransferSource.Document(id = id, title = title, depth = depth)
}

internal fun documentItemActionsSheet(
  item: EntityListItem.Document,
  onAction: (FolderAction) -> Unit,
): SheetPresentation<Unit> = sheetPresentation {
  val entityIcon =
    resolveEntityIconAppearance(
      iconName = item.iconName,
      iconColor = item.iconColor,
      fallbackIcon = Lucide.File,
      fallbackTint = AppTheme.colors.textSecondary,
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
          title = item.title,
          icon = entityIcon.icon,
          modifier = Modifier.padding(horizontal = MenuSheetHorizontalPadding),
          iconTint = entityIcon.tint,
        ) {
          EntityBreadcrumb(segments = item.breadcrumbNames())
          EntitySupportingText(
            text = visibility.label,
            color = if (visibility.isShared) AppTheme.colors.brand else AppTheme.colors.textMuted,
          )
          EntitySupportingText(text = "총 ${item.characterCount.comma}자")
        }

        Divider(color = AppTheme.colors.borderDefault)
      }
    },
  ) {
    Column(modifier = Modifier.fillMaxWidth().padding(MenuSheetActionContentPadding)) {
      folderPrimaryActionSections().forEachIndexed { index, section ->
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
              dismiss(SheetDismissReason.Programmatic)
              onAction(action.action)
            },
          )
        }
      }
    }
  }
}
