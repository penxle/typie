package co.typie.domain.entity

import androidx.compose.ui.unit.Dp
import co.typie.graphql.fragment.EntityRow_entity

data class EntityReorderOrders(val lowerOrder: String?, val upperOrder: String?)

data class EntityContainerSelectionState(
  val isSelecting: Boolean = false,
  val selectedIds: Set<String> = emptySet(),
)

data class EntityContainerSelectionSummary(
  val selectedItems: List<EntityRow_entity>,
  val folderItems: List<EntityRow_entity>,
  val documentItems: List<EntityRow_entity>,
  val commonIconName: String?,
  val commonIconColor: String?,
)

data class EntityContainerBottomOverlayMetrics(val occupiedHeight: Dp, val reservedSpacerHeight: Dp)
