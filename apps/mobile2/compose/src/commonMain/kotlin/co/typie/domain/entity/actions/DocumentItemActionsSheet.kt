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
import co.typie.graphql.QueryState
import co.typie.graphql.fragment.EntityDetails_entity
import co.typie.graphql.fragment.EntityRow_entity
import co.typie.ui.component.Divider
import co.typie.ui.component.EntityBreadcrumb
import co.typie.ui.component.EntityHeader
import co.typie.ui.component.EntitySupportingText
import co.typie.ui.component.sheet.SheetActionRow
import co.typie.ui.component.sheet.SheetLayout
import co.typie.ui.component.sheet.SheetPadding
import co.typie.ui.component.sheet.SheetScope
import co.typie.ui.component.sheet.dismiss
import co.typie.ui.skeleton.Skeleton
import co.typie.ui.theme.AppTheme

private val MenuSheetHorizontalPadding = 24.dp
private val MenuSheetActionContentPadding =
  PaddingValues(horizontal = MenuSheetHorizontalPadding, vertical = 8.dp)
private val MenuSheetRowPadding = PaddingValues(vertical = 12.dp)

@Composable
context(_: SheetScope<Unit>)
internal fun DocumentItemActionsSheet(
  entity: EntityRow_entity,
  siteName: String? = null,
  onAction: (EntityAction) -> Unit,
) {
  val detailsState = rememberEntityItemActionsState(entity.id)
  DocumentItemActionsSheetContent(
    entity = entity,
    details = (detailsState as? QueryState.Success)?.data?.entity?.entityDetails_entity,
    siteName = siteName,
    isLoadingDetails = detailsState is QueryState.Loading,
    onAction = onAction,
  )
}

@Composable
context(_: SheetScope<Unit>)
private fun DocumentItemActionsSheetContent(
  entity: EntityRow_entity,
  details: EntityDetails_entity?,
  siteName: String?,
  isLoadingDetails: Boolean,
  onAction: (EntityAction) -> Unit,
) {
  val document =
    entity.document
      ?: run {
        EntityItemActionsStatusContent(message = "문서 정보를 표시할 수 없어요.")
        return
      }
  val entityIcon = entity.entityIcon_entity.iconAppearance
  val breadcrumbNames =
    details?.breadcrumbNames(siteName)
      ?: siteName?.takeIf(String::isNotBlank)?.let(::listOf).orEmpty()
  val visibility = details?.let(::entityVisibilityPresentation)
  val characterCount = details?.document?.characterCount

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
          title = formatDocumentTitle(document.title),
          icon = entityIcon.icon,
          modifier = Modifier.padding(horizontal = MenuSheetHorizontalPadding),
          iconTint = entityIcon.tint,
        ) {
          if (breadcrumbNames.isNotEmpty() || isLoadingDetails) {
            Skeleton(enabled = isLoadingDetails) {
              EntityBreadcrumb(
                segments =
                  if (breadcrumbNames.isNotEmpty()) breadcrumbNames else listOf(Skeleton.text(4..8))
              )
            }
          }
          if (visibility != null || isLoadingDetails) {
            Skeleton(enabled = isLoadingDetails) {
              EntitySupportingText(
                text = visibility?.label ?: Skeleton.text(4..8),
                color =
                  if (visibility?.isShared == true) AppTheme.colors.brand
                  else AppTheme.colors.textMuted,
              )
            }
          }
          if (characterCount != null || isLoadingDetails) {
            Skeleton(enabled = isLoadingDetails) {
              EntitySupportingText(
                text = characterCount?.let { "총 ${it.comma}자" } ?: Skeleton.text(6..10)
              )
            }
          }
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
