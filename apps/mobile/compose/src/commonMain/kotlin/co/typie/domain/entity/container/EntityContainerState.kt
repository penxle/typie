package co.typie.domain.entity

import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import co.typie.graphql.fragment.EntityRow_entity

class EntityContainerSelection(
  val state: EntityContainerSelectionState,
  val summary: EntityContainerSelectionSummary,
  private val onStateChange: (EntityContainerSelectionState) -> Unit,
) {
  val isSelectionBarVisible: Boolean = state.isSelecting && summary.selectedItems.isNotEmpty()

  fun start(initialIds: Set<String> = emptySet()) {
    onStateChange(startEntityContainerSelection(initialIds))
  }

  fun clear() {
    onStateChange(clearEntityContainerSelection(state))
  }

  fun reset() {
    onStateChange(EntityContainerSelectionState())
  }

  fun toggle(itemId: String) {
    onStateChange(toggleEntityContainerSelection(state, itemId))
  }
}

@Composable
fun rememberEntityContainerSelection(items: List<EntityRow_entity>): EntityContainerSelection {
  var state by remember { mutableStateOf(EntityContainerSelectionState()) }
  val summary =
    remember(items, state.selectedIds) {
      resolveEntityContainerSelectionSummary(items, state.selectedIds)
    }

  return remember(state, summary) { EntityContainerSelection(state, summary) { state = it } }
}
