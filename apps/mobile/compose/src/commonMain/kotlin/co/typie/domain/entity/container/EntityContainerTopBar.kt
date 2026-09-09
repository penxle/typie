package co.typie.domain.entity

import androidx.compose.runtime.Composable
import co.typie.icons.Lucide
import co.typie.ui.component.popover.PopoverMenu
import co.typie.ui.component.topbar.TopBarButton
import co.typie.ui.icon.IconData

internal val EntityContainerTopBarTrailingKey = Any()

data class EntityContainerEditAction(
  val icon: IconData,
  val label: String,
  val onClick: () -> Unit = {},
)

@Composable
fun EntityContainerTopBarTrailing(
  isReordering: Boolean,
  isSelecting: Boolean = false,
  actions: List<EntityContainerEditAction>,
  onDoneClick: suspend () -> Unit,
  onCloseSelectionClick: suspend () -> Unit = {},
) {
  if (isReordering) {
    TopBarButton(icon = Lucide.Check, contentDescription = "완료", onClick = onDoneClick)
  } else if (isSelecting) {
    TopBarButton(icon = Lucide.X, contentDescription = "선택 취소", onClick = onCloseSelectionClick)
  } else {
    EntityContainerEditMenu(actions = actions)
  }
}

@Composable
private fun EntityContainerEditMenu(actions: List<EntityContainerEditAction>) {
  PopoverMenu(anchor = { TopBarButton(icon = Lucide.LayoutList, contentDescription = "보기 옵션") }) {
    actions.forEach { action ->
      item(icon = action.icon, label = action.label) { action.onClick() }
    }
  }
}
