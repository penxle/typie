package co.typie.domain.note

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.runtime.Composable
import androidx.compose.ui.Modifier
import androidx.compose.ui.unit.dp
import co.typie.domain.entity.displayTitle
import co.typie.domain.entity.iconAppearance
import co.typie.domain.entity.isFolder
import co.typie.graphql.fragment.NoteLinkedEntity_entity
import co.typie.icons.Lucide
import co.typie.ui.component.Divider
import co.typie.ui.component.EntityHeader
import co.typie.ui.component.EntitySupportingText
import co.typie.ui.component.sheet.SheetActionRow
import co.typie.ui.component.sheet.SheetLayout
import co.typie.ui.component.sheet.SheetPadding
import co.typie.ui.component.sheet.SheetScope
import co.typie.ui.component.sheet.dismiss
import co.typie.ui.theme.AppTheme

private val NoteLinkedEntityActionsSheetHorizontalPadding = 24.dp

@Composable
context(_: SheetScope<Unit>)
internal fun NoteLinkedEntityActionsSheet(
  linkedEntity: NoteLinkedEntity_entity,
  onOpen: () -> Unit,
  onUnlink: () -> Unit,
) {
  val entity = linkedEntity.entity
  val iconAppearance = entity.entityIcon_entity.iconAppearance
  val openLabel = if (entity.isFolder()) "이 폴더 열기" else "이 문서 열기"
  val openIcon = if (entity.isFolder()) Lucide.FolderInput else Lucide.FileInput

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
          title = entity.displayTitle(),
          icon = iconAppearance.icon,
          modifier = Modifier.padding(horizontal = NoteLinkedEntityActionsSheetHorizontalPadding),
          iconTint = iconAppearance.tint,
        ) {
          EntitySupportingText(text = if (entity.isFolder()) "폴더" else "문서")
        }

        Divider(color = AppTheme.colors.borderDefault)
      }
    },
  ) {
    Column(
      modifier =
        Modifier.fillMaxWidth()
          .padding(horizontal = NoteLinkedEntityActionsSheetHorizontalPadding, vertical = 8.dp)
    ) {
      SheetActionRow(
        icon = openIcon,
        label = openLabel,
        contentPadding = PaddingValues(vertical = 12.dp),
        onClick = {
          dismiss()
          onOpen()
        },
      )

      SheetActionRow(
        icon = Lucide.Unlink,
        label = "연결 해제",
        contentPadding = PaddingValues(vertical = 12.dp),
        tint = AppTheme.colors.danger,
        onClick = {
          dismiss()
          onUnlink()
        },
      )
    }
  }
}
