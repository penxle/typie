package co.typie.domain.entity

import androidx.compose.runtime.Composable
import androidx.compose.ui.Modifier
import co.typie.graphql.fragment.EntityItemActionsHeader_entity
import co.typie.graphql.fragment.EntityRow_entity
import co.typie.ui.theme.AppTheme

@Composable
internal fun EntityItemActionsHeader(
  entity: EntityItemActionsHeader_entity,
  isLoading: Boolean,
  modifier: Modifier = Modifier,
  extraSupportingContent: (EntityHeaderScope.(EntityRow_entity) -> Unit)? = null,
) {
  val rowEntity = entity.entityRow_entity
  val breadcrumbEntity = entity.entityBreadcrumb_entity
  val visibility = entityVisibilityPresentation(entity.visibility, entity.availability)
  val visibilityColor =
    if (visibility.isShared) AppTheme.colors.brand else AppTheme.colors.textMuted

  EntityHeader(
    title = rowEntity.displayTitle(),
    entityIcon = rowEntity.entityIcon_entity,
    modifier = modifier,
  ) {
    breadcrumb(entity = breadcrumbEntity, loading = isLoading)
    supportingText(text = visibility.label, color = visibilityColor, loading = isLoading)

    extraSupportingContent?.let { content -> content(rowEntity) }
  }
}
