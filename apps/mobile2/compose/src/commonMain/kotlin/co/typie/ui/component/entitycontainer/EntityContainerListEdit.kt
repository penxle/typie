package co.typie.ui.component.entitycontainer

import androidx.compose.animation.animateBounds
import androidx.compose.animation.core.Spring
import androidx.compose.animation.core.animateDpAsState
import androidx.compose.animation.core.animateFloatAsState
import androidx.compose.animation.core.spring
import androidx.compose.animation.core.tween
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.ColumnScope
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.key
import androidx.compose.runtime.remember
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.graphicsLayer
import androidx.compose.ui.layout.LookaheadScope
import androidx.compose.ui.platform.LocalDensity
import androidx.compose.ui.unit.Dp
import androidx.compose.ui.unit.dp
import androidx.compose.ui.zIndex
import co.typie.icons.Lucide
import co.typie.ui.component.CardDivider
import co.typie.ui.component.CardSurface
import co.typie.ui.component.EntityListCard
import co.typie.ui.component.EntityListDocumentRow
import co.typie.ui.component.EntityListFolderRow
import co.typie.ui.component.EntityListItem
import co.typie.ui.component.Text
import co.typie.ui.component.popover.Popover
import co.typie.ui.component.popover.PopoverDefaults
import co.typie.ui.component.popover.PopoverList
import co.typie.ui.component.popover.PopoverListItem
import co.typie.ui.component.popover.PopoverPlacement
import co.typie.ui.component.popover.PopoverScope
import co.typie.ui.component.popover.close
import co.typie.ui.component.reorder.ReorderCommit
import co.typie.ui.component.reorder.ReorderableListState
import co.typie.ui.component.reorder.reorderableDragHandle
import co.typie.ui.component.reorder.reorderableItem
import co.typie.ui.component.topbar.TopBarButton
import co.typie.ui.icon.Icon
import co.typie.ui.icon.IconData
import co.typie.ui.theme.AppTheme

internal val EntityContainerTopBarTrailingKey = Any()

private val EntityContainerReorderHandleWidth = 44.dp

data class EntityContainerEditAction(
  val icon: IconData,
  val label: String,
  val onClick: (closePopover: () -> Unit) -> Unit = { closePopover -> closePopover() },
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
    TopBarButton(icon = Lucide.Check, onClick = onDoneClick)
  } else if (isSelecting) {
    TopBarButton(icon = Lucide.X, onClick = onCloseSelectionClick)
  } else {
    EntityContainerEditMenu(actions = actions)
  }
}

@Composable
fun EntityContainerListContent(
  items: List<OrderedEntityItem>,
  emptyMessage: String,
  isReordering: Boolean,
  reorderState: ReorderableListState<String>,
  isPersistingReorder: Boolean,
  selectionState: EntityContainerSelectionState = EntityContainerSelectionState(),
  dimmedItemIds: Set<String> = emptySet(),
  bottomSpacerHeight: Dp = 140.dp,
  modifier: Modifier = Modifier,
  header: @Composable ColumnScope.() -> Unit = {},
  onDocumentClick: suspend (slug: String) -> Unit,
  onDocumentLongPress: (suspend (item: EntityListItem.Document) -> Unit)? = null,
  onFolderClick: suspend (entityId: String) -> Unit,
  onFolderLongPress: (suspend (item: EntityListItem.Folder) -> Unit)? = null,
  onSelectionToggle: suspend (itemId: String) -> Unit = {},
  onDragStarted: () -> Unit,
  onDragMoved: () -> Unit,
  onDragStopped: (ReorderCommit<String>?) -> Unit,
) {
  Column(modifier = modifier.fillMaxSize()) {
    header()

    if (isReordering) {
      EntityContainerReorderListCard(
        items = items,
        reorderState = reorderState,
        isPersistingReorder = isPersistingReorder,
        modifier = Modifier.padding(horizontal = 16.dp),
        onDragStarted = onDragStarted,
        onDragMoved = onDragMoved,
        onDragStopped = onDragStopped,
      )
    } else {
      EntityListCard(
        items = items.map { it.item },
        emptyMessage = emptyMessage,
        selectionState = selectionState,
        dimmedItemIds = dimmedItemIds,
        modifier = Modifier.padding(horizontal = 16.dp),
        onDocumentClick = onDocumentClick,
        onDocumentLongPress = onDocumentLongPress,
        onFolderClick = onFolderClick,
        onFolderLongPress = onFolderLongPress,
        onSelectionToggle = onSelectionToggle,
      )
    }

    Spacer(Modifier.height(bottomSpacerHeight))
  }
}

@Composable
fun EntityContainerReorderListCard(
  items: List<OrderedEntityItem>,
  reorderState: ReorderableListState<String>,
  isPersistingReorder: Boolean,
  onDragStarted: () -> Unit,
  onDragMoved: () -> Unit,
  onDragStopped: (ReorderCommit<String>?) -> Unit,
  modifier: Modifier = Modifier,
) {
  LookaheadScope {
    val boundsTransform = remember {
      androidx.compose.animation.BoundsTransform { _, _ ->
        spring(dampingRatio = 0.9f, stiffness = Spring.StiffnessMedium)
      }
    }

    Column(modifier = modifier.fillMaxWidth(), verticalArrangement = Arrangement.spacedBy(0.dp)) {
      items.forEachIndexed { index, child ->
        key(child.id) {
          val isDragging = reorderState.isDragging(child.id)

          EntityContainerReorderRow(
            modifier =
              Modifier.animateBounds(
                  lookaheadScope = this@LookaheadScope,
                  boundsTransform = boundsTransform,
                )
                .reorderableItem(state = reorderState, key = child.id),
            item = child,
            isDragging = isDragging,
            isFirst = index == 0,
            isLast = index == items.lastIndex,
            dragHandleModifier =
              Modifier.reorderableDragHandle(
                state = reorderState,
                key = child.id,
                enabled = !isPersistingReorder,
                onDragStarted = onDragStarted,
                onDragMoved = onDragMoved,
                onDragStopped = onDragStopped,
              ),
          )
        }
      }
    }
  }
}

@Composable
private fun EntityContainerEditMenu(actions: List<EntityContainerEditAction>) {
  Popover(
    placement = PopoverPlacement.BelowEnd,
    anchor = { TopBarButton(icon = Lucide.LayoutList) },
    pane = {
      Column(modifier = Modifier.padding(PopoverDefaults.PanePadding)) {
        EntityContainerEditActionList(actions = actions)
      }
    },
  )
}

@Composable
context(_: PopoverScope)
private fun EntityContainerEditActionList(actions: List<EntityContainerEditAction>) {
  PopoverList(
    items =
      actions.map { action ->
        PopoverListItem(
          content = {
            EntityContainerEditActionRow(
              action = action,
              modifier = Modifier.height(42.dp).padding(horizontal = 16.dp),
            )
          },
          onSelected = { action.onClick { close() } },
        )
      }
  )
}

@Composable
private fun EntityContainerEditActionRow(
  action: EntityContainerEditAction,
  modifier: Modifier = Modifier,
) {
  Row(modifier = modifier, verticalAlignment = Alignment.CenterVertically) {
    Icon(icon = action.icon, modifier = Modifier.size(18.dp), tint = AppTheme.colors.textPrimary)

    Spacer(Modifier.width(12.dp))

    Text(
      text = action.label,
      modifier = Modifier.weight(1f),
      style = AppTheme.typography.action,
      color = AppTheme.colors.textPrimary,
    )
  }
}

@Composable
private fun EntityContainerReorderRow(
  modifier: Modifier,
  item: OrderedEntityItem,
  isDragging: Boolean,
  isFirst: Boolean,
  isLast: Boolean,
  dragHandleModifier: Modifier = Modifier,
) {
  val topStartRadius by
    animateDpAsState(
      targetValue = if (isFirst) 12.dp else 0.dp,
      animationSpec = tween(durationMillis = 140),
    )
  val topEndRadius by
    animateDpAsState(
      targetValue = if (isFirst) 12.dp else 0.dp,
      animationSpec = tween(durationMillis = 140),
    )
  val bottomStartRadius by
    animateDpAsState(
      targetValue = if (isLast) 12.dp else 0.dp,
      animationSpec = tween(durationMillis = 140),
    )
  val bottomEndRadius by
    animateDpAsState(
      targetValue = if (isLast) 12.dp else 0.dp,
      animationSpec = tween(durationMillis = 140),
    )
  val shape =
    RoundedCornerShape(
      topStart = topStartRadius,
      topEnd = topEndRadius,
      bottomStart = bottomStartRadius,
      bottomEnd = bottomEndRadius,
    )
  val density = LocalDensity.current
  val animatedScale by
    animateFloatAsState(
      targetValue = if (isDragging) 1.008f else 1f,
      animationSpec =
        if (isDragging) {
          tween(durationMillis = 120)
        } else {
          spring(dampingRatio = 0.72f, stiffness = Spring.StiffnessMediumLow)
        },
    )
  val animatedElevation by
    animateDpAsState(
      targetValue = if (isDragging) 3.dp else 0.dp,
      animationSpec =
        if (isDragging) {
          tween(durationMillis = 120)
        } else {
          spring(dampingRatio = 0.72f, stiffness = Spring.StiffnessMediumLow)
        },
    )

  CardSurface(
    modifier =
      modifier
        .fillMaxWidth()
        .graphicsLayer {
          scaleX = animatedScale
          scaleY = animatedScale
          shadowElevation = with(density) { animatedElevation.toPx() }
          this.shape = shape
          clip = false
        }
        .zIndex(if (isDragging) 1f else 0f),
    shape = shape,
    color = if (isDragging) AppTheme.colors.surfaceRaised else AppTheme.colors.surfaceDefault,
  ) {
    Column(modifier = Modifier.fillMaxWidth()) {
      if (!isFirst) {
        CardDivider(inset = 20.dp)
      }

      Row(modifier = Modifier.fillMaxWidth(), verticalAlignment = Alignment.CenterVertically) {
        Box(
          modifier =
            dragHandleModifier.size(width = EntityContainerReorderHandleWidth, height = 56.dp),
          contentAlignment = Alignment.Center,
        ) {
          Icon(
            icon = Lucide.GripVertical,
            modifier = Modifier.size(18.dp),
            tint = AppTheme.colors.textTertiary,
          )
        }

        when (val child = item.item) {
          is EntityListItem.Document ->
            EntityListDocumentRow(
              item = child,
              modifier = Modifier.weight(1f),
              interactive = false,
              onClick = {},
            )

          is EntityListItem.Folder ->
            EntityListFolderRow(
              item = child,
              modifier = Modifier.weight(1f),
              interactive = false,
              onClick = {},
            )
        }
      }
    }
  }
}
