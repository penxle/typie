package co.typie.domain.entity

import androidx.compose.animation.animateBounds
import androidx.compose.animation.core.Spring
import androidx.compose.animation.core.animateDpAsState
import androidx.compose.animation.core.animateFloatAsState
import androidx.compose.animation.core.spring
import androidx.compose.animation.core.tween
import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.ColumnScope
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxHeight
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
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.graphicsLayer
import androidx.compose.ui.platform.LocalDensity
import androidx.compose.ui.unit.Dp
import androidx.compose.ui.unit.dp
import androidx.compose.ui.zIndex
import co.typie.datetime.timeAgo
import co.typie.ext.separated
import co.typie.graphql.fragment.EntityRow_entity
import co.typie.icons.Lucide
import co.typie.ui.component.CardDivider
import co.typie.ui.component.CardSurface
import co.typie.ui.component.Text
import co.typie.ui.component.reorder.ReorderDrop
import co.typie.ui.component.reorder.ReorderableColumn
import co.typie.ui.component.reorder.ReorderableColumnState
import co.typie.ui.component.reorder.reorderableDragHandle
import co.typie.ui.component.reorder.reorderableItem
import co.typie.ui.icon.Icon
import co.typie.ui.theme.AppShapes
import co.typie.ui.theme.AppTheme

@Composable
fun EntityContainerListContent(
  items: List<EntityRow_entity>,
  emptyMessage: String,
  isReordering: Boolean,
  reorderState: ReorderableColumnState<String>,
  selectionState: EntityContainerSelectionState = EntityContainerSelectionState(),
  dimmedItemIds: Set<String> = emptySet(),
  bottomSpacerHeight: Dp = EntityBottomOverlayDefaults.DefaultBottomSpacerHeight,
  modifier: Modifier = Modifier,
  header: @Composable ColumnScope.() -> Unit = {},
  onDocumentClick: suspend (entityId: String) -> Unit,
  onDocumentLongPress: (suspend (item: EntityRow_entity) -> Unit)? = null,
  onFolderClick: suspend (entityId: String) -> Unit,
  onFolderLongPress: (suspend (item: EntityRow_entity) -> Unit)? = null,
  onSelectionToggle: suspend (itemId: String) -> Unit = {},
  onDragStarted: () -> Unit = {},
  onDragMoved: () -> Unit = {},
  onDragStopped: (ReorderDrop<String>?) -> Unit,
) {
  Column(modifier = modifier.fillMaxSize()) {
    header()

    if (isReordering) {
      EntityContainerReorderListCard(
        items = items,
        reorderState = reorderState,
        modifier = Modifier.padding(horizontal = 16.dp),
        onDragStarted = onDragStarted,
        onDragMoved = onDragMoved,
        onDragStopped = onDragStopped,
      )
    } else if (items.isEmpty()) {
      Box(
        modifier =
          Modifier.padding(horizontal = 16.dp)
            .fillMaxWidth()
            .height(110.dp)
            .background(AppTheme.colors.surfaceDefault, AppShapes.rounded(AppShapes.md)),
        contentAlignment = Alignment.Center,
      ) {
        Text(emptyMessage, style = AppTheme.typography.action, color = AppTheme.colors.textTertiary)
      }
    } else {
      CardSurface(modifier = Modifier.padding(horizontal = 16.dp).fillMaxWidth()) {
        Column(Modifier.fillMaxWidth()) {
          items.separated(separator = { CardDivider() }) { entity ->
            EntityContainerItemRow(
              entity = entity,
              selected = entity.id in selectionState.selectedIds,
              showSelectionControls = selectionState.isSelecting,
              opacity = if (entity.id in dimmedItemIds) 0.5f else 1f,
              onLongPress =
                when {
                  entity.document != null ->
                    onDocumentLongPress?.let { handler -> { handler(entity) } }
                  entity.folder != null -> onFolderLongPress?.let { handler -> { handler(entity) } }
                  else -> null
                },
              onClick = {
                if (selectionState.isSelecting) {
                  onSelectionToggle(entity.id)
                } else {
                  when {
                    entity.document != null -> onDocumentClick(entity.id)
                    entity.folder != null -> onFolderClick(entity.id)
                  }
                }
              },
            )
          }
        }
      }
    }

    Spacer(Modifier.height(bottomSpacerHeight))
  }
}

@Composable
fun EntityContainerReorderListCard(
  items: List<EntityRow_entity>,
  reorderState: ReorderableColumnState<String>,
  onDragStarted: () -> Unit = {},
  onDragMoved: () -> Unit = {},
  onDragStopped: (ReorderDrop<String>?) -> Unit,
  modifier: Modifier = Modifier,
) {
  ReorderableColumn(
    state = reorderState,
    modifier = modifier.fillMaxWidth(),
    verticalArrangement = Arrangement.spacedBy(0.dp),
  ) {
    val boundsTransform = remember {
      androidx.compose.animation.BoundsTransform { _, _ ->
        spring(dampingRatio = 0.9f, stiffness = Spring.StiffnessMedium)
      }
    }

    items.forEachIndexed { index, entity ->
      key(entity.id) {
        val isDragging = reorderState.isDragging(entity.id)

        val rowModifier =
          if (isDragging) {
            Modifier
          } else {
            Modifier.animateBounds(
              lookaheadScope = this@ReorderableColumn,
              boundsTransform = boundsTransform,
            )
          }

        EntityContainerReorderRow(
          modifier = rowModifier.reorderableItem(state = reorderState, key = entity.id),
          item = entity,
          isDragging = isDragging,
          isFirst = index == 0,
          isLast = index == items.lastIndex,
          dragHandleModifier =
            Modifier.reorderableDragHandle(
              state = reorderState,
              key = entity.id,
              onDragStarted = onDragStarted,
              onDragMoved = onDragMoved,
              onDragStopped = onDragStopped,
            ),
        )
      }
    }
  }
}

@Composable
private fun EntityContainerReorderRow(
  modifier: Modifier,
  item: EntityRow_entity,
  isDragging: Boolean,
  isFirst: Boolean,
  isLast: Boolean,
  dragHandleModifier: Modifier = Modifier,
) {
  val topStartRadius by
    animateDpAsState(
      targetValue = if (isFirst) AppShapes.md else 0.dp,
      animationSpec = tween(durationMillis = 140),
    )
  val topEndRadius by
    animateDpAsState(
      targetValue = if (isFirst) AppShapes.md else 0.dp,
      animationSpec = tween(durationMillis = 140),
    )
  val bottomStartRadius by
    animateDpAsState(
      targetValue = if (isLast) AppShapes.md else 0.dp,
      animationSpec = tween(durationMillis = 140),
    )
  val bottomEndRadius by
    animateDpAsState(
      targetValue = if (isLast) AppShapes.md else 0.dp,
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

      Box(modifier = Modifier.fillMaxWidth()) {
        EntityContainerItemRow(
          modifier = Modifier.fillMaxWidth(),
          entity = item,
          interactive = false,
          leading = { EntityContainerReorderGrip() },
          onClick = {},
        )

        Box(modifier = Modifier.matchParentSize()) {
          Box(
            modifier = dragHandleModifier.align(Alignment.CenterStart).fillMaxHeight().width(44.dp)
          )
        }
      }
    }
  }
}

@Composable
private fun EntityContainerReorderGrip(modifier: Modifier = Modifier) {
  Icon(
    icon = Lucide.GripVertical,
    modifier = modifier.size(18.dp),
    tint = AppTheme.colors.textTertiary,
  )
}

@Composable
private fun EntityContainerItemRow(
  entity: EntityRow_entity,
  modifier: Modifier = Modifier,
  enabled: Boolean = true,
  interactive: Boolean = enabled,
  opacity: Float = 1f,
  selected: Boolean = false,
  showSelectionControls: Boolean = false,
  leading: (@Composable () -> Unit)? = null,
  onLongPress: (suspend () -> Unit)? = null,
  onClick: suspend () -> Unit,
) {
  EntityRow(
    entity = entity,
    modifier = modifier,
    enabled = enabled,
    interactive = interactive,
    opacity = opacity,
    backgroundColor =
      if (showSelectionControls && selected) AppTheme.colors.brandSubtle else Color.Transparent,
    leading =
      if (showSelectionControls) {
        { EntityRowSelectionControl(selected = selected) }
      } else {
        leading
      },
    trailing =
      when {
        showSelectionControls -> null
        entity.folder != null -> {
          { EntityRowChevron() }
        }
        else -> null
      },
    onLongPress = onLongPress,
    onClick = onClick,
  ) {
    when {
      entity.document != null -> {
        val document = requireNotNull(entity.document)
        documentTitle(document = document, trailingText = document.updatedAt.timeAgo())
        documentExcerpt(document = document)
      }
      entity.folder != null -> {
        val folder = requireNotNull(entity.folder)
        folderTitle(folder = folder)
        folderSummary(folder = folder)
      }
    }
  }
}
