package co.typie.domain.entity

import androidx.compose.animation.core.Spring
import androidx.compose.animation.core.animateDpAsState
import androidx.compose.animation.core.animateFloatAsState
import androidx.compose.animation.core.spring
import androidx.compose.animation.core.tween
import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxHeight
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.lazy.itemsIndexed
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.graphicsLayer
import androidx.compose.ui.unit.Dp
import androidx.compose.ui.unit.dp
import androidx.compose.ui.zIndex
import co.typie.datetime.timeAgo
import co.typie.domain.goal.EntityGoalGlyph
import co.typie.graphql.fragment.EntityGoalGlyph_entity
import co.typie.graphql.fragment.EntityRow_entity
import co.typie.icons.Lucide
import co.typie.ui.component.CardDivider
import co.typie.ui.component.CardSurface
import co.typie.ui.component.Text
import co.typie.ui.component.reorder.ReorderDrop
import co.typie.ui.component.reorder.ReorderableLazyColumn
import co.typie.ui.component.reorder.ReorderableLazyColumnState
import co.typie.ui.component.reorder.reorderableAnimatedItem
import co.typie.ui.component.reorder.reorderableDragHandle
import co.typie.ui.component.reorder.reorderableItem
import co.typie.ui.component.reorder.reorderableViewport
import co.typie.ui.icon.Icon
import co.typie.ui.theme.AppShapes
import co.typie.ui.theme.AppTheme
import co.typie.ui.theme.shadow

@Composable
fun EntityContainerListContent(
  items: List<EntityRow_entity>,
  emptyMessage: String,
  isReordering: Boolean,
  reorderState: ReorderableLazyColumnState<String>,
  selectionState: EntityContainerSelectionState = EntityContainerSelectionState(),
  dimmedItemIds: Set<String> = emptySet(),
  goalGlyphs: Map<String, EntityGoalGlyph_entity> = emptyMap(),
  bottomSpacerHeight: Dp = EntityBottomOverlayDefaults.DefaultBottomSpacerHeight,
  viewportTopInset: Dp = 0.dp,
  viewportBottomInset: Dp = 0.dp,
  modifier: Modifier = Modifier,
  contentPadding: PaddingValues = PaddingValues(0.dp),
  header: (@Composable () -> Unit)? = null,
  onDocumentClick: suspend (entityId: String) -> Unit,
  onDocumentLongPress: (suspend (item: EntityRow_entity) -> Unit)? = null,
  onFolderClick: suspend (entityId: String) -> Unit,
  onFolderLongPress: (suspend (item: EntityRow_entity) -> Unit)? = null,
  onSelectionToggle: suspend (itemId: String) -> Unit = {},
  onDragStarted: () -> Unit = {},
  onDragMoved: () -> Unit = {},
  onDragStopped: (ReorderDrop<String>?) -> Unit,
) {
  ReorderableLazyColumn(
    state = reorderState,
    modifier =
      modifier
        .fillMaxSize()
        .reorderableViewport(
          state = reorderState,
          viewportTopInset = viewportTopInset,
          viewportBottomInset = viewportBottomInset,
        ),
    contentPadding = contentPadding,
  ) {
    if (header != null) {
      item(key = EntityContainerHeaderKey) { header() }
    }

    if (isReordering) {
      itemsIndexed(items = items, key = { _, entity -> entity.id }) { _, entity ->
        val projectedIndex = reorderState.keys.indexOf(entity.id)
        val isDragging = reorderState.isDragging(entity.id)
        EntityContainerReorderRow(
          modifier =
            reorderableAnimatedItem(state = reorderState, key = entity.id)
              .reorderableItem(state = reorderState, key = entity.id),
          item = entity,
          goal = goalGlyphs[entity.id],
          isDragging = isDragging,
          isFirst = projectedIndex == 0,
          isLast = projectedIndex == reorderState.keys.lastIndex,
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
    } else if (items.isEmpty()) {
      item(key = EntityContainerEmptyKey) {
        Box(
          modifier =
            Modifier.fillMaxWidth()
              .height(110.dp)
              .background(AppTheme.colors.surfaceDefault, AppShapes.rounded(AppShapes.md)),
          contentAlignment = Alignment.Center,
        ) {
          Text(emptyMessage, style = AppTheme.typography.action, color = AppTheme.colors.textMuted)
        }
      }
    } else {
      itemsIndexed(items = items, key = { _, entity -> entity.id }) { index, entity ->
        EntityContainerNormalRow(
          entity = entity,
          goal = goalGlyphs[entity.id],
          isFirst = index == 0,
          isLast = index == items.lastIndex,
          selected = entity.id in selectionState.selectedIds,
          showSelectionControls = selectionState.isSelecting,
          opacity = if (entity.id in dimmedItemIds) 0.5f else 1f,
          onLongPress =
            when {
              entity.document != null -> onDocumentLongPress?.let { handler -> { handler(entity) } }
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

    item(key = EntityContainerBottomSpacerKey) { Spacer(Modifier.height(bottomSpacerHeight)) }
  }
}

@Composable
private fun EntityContainerNormalRow(
  entity: EntityRow_entity,
  goal: EntityGoalGlyph_entity?,
  isFirst: Boolean,
  isLast: Boolean,
  selected: Boolean,
  showSelectionControls: Boolean,
  opacity: Float,
  onLongPress: (suspend () -> Unit)?,
  onClick: suspend () -> Unit,
) {
  val shape =
    RoundedCornerShape(
      topStart = if (isFirst) AppShapes.md else 0.dp,
      topEnd = if (isFirst) AppShapes.md else 0.dp,
      bottomStart = if (isLast) AppShapes.md else 0.dp,
      bottomEnd = if (isLast) AppShapes.md else 0.dp,
    )
  CardSurface(modifier = Modifier.fillMaxWidth(), shape = shape) {
    Column(Modifier.fillMaxWidth()) {
      if (!isFirst) CardDivider()
      EntityContainerItemRow(
        entity = entity,
        goal = goal,
        selected = selected,
        showSelectionControls = showSelectionControls,
        opacity = opacity,
        onLongPress = onLongPress,
        onClick = onClick,
      )
    }
  }
}

private const val EntityContainerHeaderKey = "entity-container-header"

private const val EntityContainerEmptyKey = "entity-container-empty"

private const val EntityContainerBottomSpacerKey = "entity-container-bottom-spacer"

private val EntityGoalGlyphGap = 8.dp

@Composable
private fun EntityContainerReorderRow(
  modifier: Modifier,
  item: EntityRow_entity,
  goal: EntityGoalGlyph_entity?,
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
        .shadow(
          AppTheme.shadows.lg,
          shape,
          alpha = { (animatedElevation.value / 3f).coerceIn(0f, 1f) },
        )
        .graphicsLayer {
          scaleX = animatedScale
          scaleY = animatedScale
        }
        .zIndex(if (isDragging) 1f else 0f),
    shape = shape,
    color = if (isDragging) AppTheme.colors.surfaceDefault else AppTheme.colors.surfaceDefault,
  ) {
    Column(modifier = Modifier.fillMaxWidth()) {
      if (!isFirst) {
        CardDivider(inset = 20.dp)
      }

      Box(modifier = Modifier.fillMaxWidth()) {
        EntityContainerItemRow(
          modifier = Modifier.fillMaxWidth(),
          entity = item,
          goal = goal,
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
    tint = AppTheme.colors.textMuted,
  )
}

@Composable
private fun EntityContainerItemRow(
  entity: EntityRow_entity,
  goal: EntityGoalGlyph_entity?,
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
      if (showSelectionControls && selected) AppTheme.colors.surfaceInset else Color.Transparent,
    leading =
      if (showSelectionControls) {
        { EntityRowSelectionControl(selected = selected) }
      } else {
        leading
      },
    trailing =
      when {
        showSelectionControls -> null
        goal != null && entity.folder != null -> {
          {
            Row(
              verticalAlignment = Alignment.CenterVertically,
              horizontalArrangement = Arrangement.spacedBy(EntityGoalGlyphGap),
            ) {
              EntityGoalGlyph(goal)
              EntityRowChevron()
            }
          }
        }
        goal != null -> {
          { EntityGoalGlyph(goal) }
        }
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
