package co.typie.domain.entity

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.unit.dp
import co.typie.icons.Lucide
import co.typie.ui.component.Divider
import co.typie.ui.component.Text
import co.typie.ui.component.entitycontainer.EntityContainerSelectionSummary
import co.typie.ui.component.sheet.SheetActionRow
import co.typie.ui.component.sheet.SheetLayout
import co.typie.ui.component.sheet.SheetPadding
import co.typie.ui.component.sheet.SheetScope
import co.typie.ui.component.sheet.dismiss
import co.typie.ui.icon.Icon
import co.typie.ui.theme.AppTheme

private val SelectionSheetHorizontalPadding = 24.dp
private val SelectionSheetActionContentPadding =
  PaddingValues(horizontal = SelectionSheetHorizontalPadding, vertical = 8.dp)
private val SelectionSheetRowPadding = PaddingValues(vertical = 12.dp)

@Composable
context(_: SheetScope<Unit>)
internal fun EntitySelectionActionsSheet(
  summary: EntityContainerSelectionSummary,
  onChangeIcon: () -> Unit,
  onShareFolders: (() -> Unit)? = null,
  onShareDocuments: (() -> Unit)? = null,
  onCopy: () -> Unit,
  onCut: () -> Unit,
  onDelete: () -> Unit,
) {
  SheetLayout(
    bodyScroll = false,
    padding = SheetPadding.None,
    verticalSpacing = 0.dp,
    header = {
      Column(
        modifier = Modifier.fillMaxWidth(),
        verticalArrangement = Arrangement.spacedBy(16.dp),
      ) {
        Column(
          modifier = Modifier.fillMaxWidth().padding(horizontal = SelectionSheetHorizontalPadding),
          verticalArrangement = Arrangement.spacedBy(6.dp),
        ) {
          Row(
            verticalAlignment = Alignment.CenterVertically,
            horizontalArrangement = Arrangement.spacedBy(12.dp),
          ) {
            Icon(
              icon = Lucide.SquareCheck,
              modifier = Modifier.size(20.dp),
              tint = AppTheme.colors.textPrimary,
            )
            Text(
              text = "${summary.selectedItems.size}개 선택됨",
              style = AppTheme.typography.title,
              color = AppTheme.colors.textPrimary,
            )
          }

          Row(
            horizontalArrangement = Arrangement.spacedBy(12.dp),
            verticalAlignment = Alignment.CenterVertically,
          ) {
            if (summary.folderItems.isNotEmpty()) {
              SelectionCountChip(icon = Lucide.Folder, label = "${summary.folderItems.size}개")
            }
            if (summary.documentItems.isNotEmpty()) {
              SelectionCountChip(icon = Lucide.File, label = "${summary.documentItems.size}개")
            }
          }
        }

        Divider(color = AppTheme.colors.borderDefault)
      }
    },
  ) {
    Column(modifier = Modifier.fillMaxWidth().padding(SelectionSheetActionContentPadding)) {
      SheetActionRow(
        icon = Lucide.Palette,
        label = "아이콘 변경",
        contentPadding = SelectionSheetRowPadding,
        onClick = {
          dismiss()
          onChangeIcon()
        },
      )

      if (summary.folderItems.isNotEmpty()) {
        SheetActionRow(
          icon = Lucide.Blend,
          label = "폴더 ${summary.folderItems.size}개 공유 및 게시",
          contentPadding = SelectionSheetRowPadding,
          onClick = {
            dismiss()
            onShareFolders?.invoke()
          },
        )
      }

      if (summary.documentItems.isNotEmpty()) {
        SheetActionRow(
          icon = Lucide.Blend,
          label = "문서 ${summary.documentItems.size}개 공유 및 게시",
          contentPadding = SelectionSheetRowPadding,
          onClick = {
            dismiss()
            onShareDocuments?.invoke()
          },
        )
      }

      SheetActionRow(
        icon = Lucide.ClipboardCopy,
        label = "복사",
        contentPadding = SelectionSheetRowPadding,
        onClick = {
          dismiss()
          onCopy()
        },
      )

      SheetActionRow(
        icon = Lucide.Scissors,
        label = "잘라내기",
        contentPadding = SelectionSheetRowPadding,
        onClick = {
          dismiss()
          onCut()
        },
      )

      SheetActionRow(
        icon = Lucide.Trash2,
        label = "삭제",
        contentPadding = SelectionSheetRowPadding,
        tint = AppTheme.colors.danger,
        onClick = {
          dismiss()
          onDelete()
        },
      )
    }
  }
}

@Composable
private fun SelectionCountChip(icon: co.typie.ui.icon.IconData, label: String) {
  Row(
    horizontalArrangement = Arrangement.spacedBy(4.dp),
    verticalAlignment = Alignment.CenterVertically,
  ) {
    Icon(icon = icon, modifier = Modifier.size(14.dp), tint = AppTheme.colors.textTertiary)
    Text(text = label, style = AppTheme.typography.caption, color = AppTheme.colors.textTertiary)
  }
}
