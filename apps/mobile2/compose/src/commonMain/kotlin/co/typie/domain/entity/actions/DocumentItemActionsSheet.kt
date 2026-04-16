package co.typie.domain.entity

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.runtime.Composable
import androidx.compose.ui.Modifier
import androidx.compose.ui.unit.dp
import co.typie.ext.comma
import co.typie.graphql.EntityItemActions_Query
import co.typie.graphql.QueryState
import co.typie.graphql.fragment.EntityRow_entity
import co.typie.ui.component.Divider
import co.typie.ui.component.sheet.SheetActionRow
import co.typie.ui.component.sheet.SheetLayout
import co.typie.ui.component.sheet.SheetPadding
import co.typie.ui.component.sheet.SheetScope
import co.typie.ui.component.sheet.dismiss
import co.typie.ui.theme.AppTheme

private val MenuSheetHorizontalPadding = 24.dp
private val MenuSheetActionContentPadding =
  PaddingValues(horizontal = MenuSheetHorizontalPadding, vertical = 8.dp)
private val MenuSheetRowPadding = PaddingValues(vertical = 12.dp)

@Composable
context(_: SheetScope<Unit>)
internal fun DocumentItemActionsSheet(entity: EntityRow_entity, onAction: (EntityAction) -> Unit) {
  val detailsQuery = rememberEntityItemActionsQuery(entity)
  DocumentItemActionsSheetContent(
    details = detailsQuery.data.entity,
    resolved = detailsQuery.state is QueryState.Success,
    onAction = onAction,
  )
}

@Composable
context(_: SheetScope<Unit>)
private fun DocumentItemActionsSheetContent(
  details: EntityItemActions_Query.Entity,
  resolved: Boolean,
  onAction: (EntityAction) -> Unit,
) {
  val entity = details.entityItemActionsHeader_entity.entityRow_entity
  entity.document
    ?: run {
      EntityItemActionsStatusContent(message = "문서 정보를 표시할 수 없어요.")
      return
    }
  val characterCount =
    if (resolved) details.documentItemActions_entity.node.onDocument?.characterCount else null

  SheetLayout(
    bodyScroll = false,
    padding = SheetPadding.None,
    verticalSpacing = 0.dp,
    header = {
      Column(
        modifier = Modifier.fillMaxWidth(),
        verticalArrangement = Arrangement.spacedBy(16.dp),
      ) {
        EntityItemActionsHeader(
          entity = details.entityItemActionsHeader_entity,
          isLoading = !resolved,
          modifier = Modifier.padding(horizontal = MenuSheetHorizontalPadding),
        ) {
          supportingText(
            text = characterCount?.let { "총 ${it.comma}자" },
            loading = !resolved,
            placeholderLength = 6..10,
          )
        }

        Divider(color = AppTheme.colors.borderDefault)
      }
    },
  ) {
    Column(modifier = Modifier.fillMaxWidth().padding(MenuSheetActionContentPadding)) {
      entityItemActionSections().forEachIndexed { index, section ->
        if (index > 0) {
          Divider()
        }

        section.items.forEach { action ->
          SheetActionRow(
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
