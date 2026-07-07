package co.typie.screen.editor.editor.toolbar.bottom

import androidx.compose.animation.core.EaseOutCubic
import androidx.compose.animation.core.LinearEasing
import androidx.compose.animation.core.tween
import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.clickable
import androidx.compose.foundation.gestures.awaitEachGesture
import androidx.compose.foundation.gestures.awaitFirstDown
import androidx.compose.foundation.interaction.MutableInteractionSource
import androidx.compose.foundation.interaction.collectIsPressedAsState
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.BoxWithConstraints
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.requiredSize
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.rememberScrollState
import androidx.compose.runtime.Composable
import androidx.compose.runtime.DisposableEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableIntStateOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.focus.focusProperties
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.input.pointer.changedToUp
import androidx.compose.ui.input.pointer.pointerInput
import androidx.compose.ui.platform.LocalDensity
import androidx.compose.ui.semantics.Role
import androidx.compose.ui.unit.dp
import co.typie.editor.ffi.InsertionOp
import co.typie.editor.ffi.Message
import co.typie.ext.InteractionScope
import co.typie.ext.LocalInteractionSource
import co.typie.ext.horizontalScroll
import co.typie.ext.pressScale
import co.typie.ext.verticalScroll
import co.typie.icons.Lucide
import co.typie.screen.editor.editor.toolbar.ToolbarHeight
import co.typie.ui.component.Text
import co.typie.ui.icon.Icon
import co.typie.ui.theme.AppShapes
import co.typie.ui.theme.AppTheme
import kotlin.math.floor
import kotlin.math.roundToInt
import kotlinx.coroutines.Job
import kotlinx.coroutines.delay
import kotlinx.coroutines.isActive
import kotlinx.coroutines.launch

@Composable
internal fun BottomToolbarTableSizeSelector(
  onEditorMessage: (Message) -> Unit,
  onEditorInputRequest: () -> Unit,
  modifier: Modifier = Modifier,
) {
  var selectedRows by remember { mutableIntStateOf(DefaultTableRows) }
  var selectedCols by remember { mutableIntStateOf(DefaultTableCols) }
  var activeTouchViewport by remember { mutableStateOf<Offset?>(null) }
  var holdResampleJob by remember { mutableStateOf<Job?>(null) }
  val horizontalScrollState = rememberScrollState()
  val verticalScrollState = rememberScrollState()
  val scope = rememberCoroutineScope()
  val density = LocalDensity.current
  val gridContentInsets = remember { PaddingValues(TableSizePanelPadding) }

  DisposableEffect(Unit) { onDispose { holdResampleJob?.cancel() } }

  BoxWithConstraints(modifier = modifier.fillMaxSize()) {
    val viewportWidthPx = with(density) { maxWidth.toPx() }
    val viewportHeightPx = with(density) { maxHeight.toPx() }
    val gridCellSizePx = with(density) { TableSizeGridCellSize.toPx() }
    val gridGapPx = with(density) { TableSizeGridGap.toPx() }
    val gridStartPaddingPx =
      with(density) { (TableSizePanelPadding + TableSizeGridOuterPadding).toPx() }

    fun centerToCell(
      rowIndex: Int,
      colIndex: Int,
      durationMillis: Int = TableSizeCenterAnimationMillis,
      useLinearEasing: Boolean = false,
    ) {
      val easing = if (useLinearEasing) LinearEasing else EaseOutCubic
      val targetX =
        tableSizeCenteredScrollTarget(
          index = colIndex,
          viewportExtent = viewportWidthPx,
          maxScroll = horizontalScrollState.maxValue,
          cellSize = gridCellSizePx,
          gap = gridGapPx,
          outerPadding = gridStartPaddingPx,
        )
      val targetY =
        tableSizeCenteredScrollTarget(
          index = rowIndex,
          viewportExtent = viewportHeightPx,
          maxScroll = verticalScrollState.maxValue,
          cellSize = gridCellSizePx,
          gap = gridGapPx,
          outerPadding = gridStartPaddingPx,
        )
      scope.launch {
        horizontalScrollState.animateScrollTo(targetX, tween(durationMillis, easing = easing))
      }
      scope.launch {
        verticalScrollState.animateScrollTo(targetY, tween(durationMillis, easing = easing))
      }
    }

    fun selectFromViewportPosition(
      viewportPosition: Offset,
      centerDurationMillis: Int = TableSizeDragCenterAnimationMillis,
      useLinearEasing: Boolean = true,
    ) {
      activeTouchViewport = viewportPosition
      val selection =
        tableSizeSelectionFromGridOffset(
          x = viewportPosition.x + horizontalScrollState.value,
          y = viewportPosition.y + verticalScrollState.value,
          cellSize = gridCellSizePx,
          gap = gridGapPx,
          outerPadding = gridStartPaddingPx,
        )
      if (selection.rows == selectedRows && selection.cols == selectedCols) {
        return
      }

      selectedRows = selection.rows
      selectedCols = selection.cols
      centerToCell(
        rowIndex = selection.rows - 1,
        colIndex = selection.cols - 1,
        durationMillis = centerDurationMillis,
        useLinearEasing = useLinearEasing,
      )
    }

    fun startHoldResample() {
      holdResampleJob?.cancel()
      holdResampleJob = scope.launch {
        while (isActive) {
          delay(TableSizeHoldResampleMillis.toLong())
          val viewportPosition = activeTouchViewport ?: break
          selectFromViewportPosition(viewportPosition)
        }
      }
    }

    fun stopHoldResample() {
      holdResampleJob?.cancel()
      holdResampleJob = null
      activeTouchViewport = null
    }

    Box(Modifier.fillMaxSize()) {
      Box(
        modifier =
          Modifier.fillMaxSize()
            .verticalScroll(verticalScrollState, enabled = false)
            .horizontalScroll(horizontalScrollState, enabled = false)
      ) {
        Box(modifier = Modifier.padding(gridContentInsets)) {
          Column(
            modifier =
              Modifier.requiredSize(TableSizeGridExtent).padding(TableSizeGridOuterPadding),
            verticalArrangement = Arrangement.spacedBy(TableSizeGridGap),
          ) {
            repeat(MaxTableSize) { row ->
              Row(horizontalArrangement = Arrangement.spacedBy(TableSizeGridGap)) {
                repeat(MaxTableSize) { col ->
                  TableSizeCell(selected = row < selectedRows && col < selectedCols)
                }
              }
            }
          }
        }
      }

      Box(
        modifier =
          Modifier.fillMaxSize().pointerInput(
            viewportWidthPx,
            viewportHeightPx,
            horizontalScrollState,
            verticalScrollState,
          ) {
            awaitEachGesture {
              val down = awaitFirstDown()
              selectFromViewportPosition(down.position)
              startHoldResample()
              down.consume()
              var released = false

              try {
                while (true) {
                  val event = awaitPointerEvent()
                  val change = event.changes.firstOrNull { it.id == down.id } ?: break
                  if (change.changedToUp()) {
                    selectFromViewportPosition(change.position)
                    change.consume()
                    released = true
                    break
                  }
                  if (!change.pressed) {
                    break
                  }
                  selectFromViewportPosition(change.position)
                  change.consume()
                }
              } finally {
                stopHoldResample()
              }

              if (released) {
                centerToCell(rowIndex = selectedRows - 1, colIndex = selectedCols - 1)
              }
            }
          }
      )

      TableInsertButton(
        rows = selectedRows,
        cols = selectedCols,
        onClick = {
          onEditorMessage(
            Message.Insertion(InsertionOp.Table(rows = selectedRows, cols = selectedCols))
          )
          onEditorInputRequest()
        },
        modifier = Modifier.align(Alignment.BottomEnd).padding(12.dp),
      )
    }
  }
}

internal data class TableSizeSelection(val rows: Int, val cols: Int)

internal fun tableSizeSelectionFromGridOffset(
  x: Float,
  y: Float,
  cellSize: Float,
  gap: Float,
  outerPadding: Float,
): TableSizeSelection {
  val stride = cellSize + gap
  val col = floor((x - outerPadding) / stride).toInt().coerceIn(0, MaxTableSize - 1)
  val row = floor((y - outerPadding) / stride).toInt().coerceIn(0, MaxTableSize - 1)
  return TableSizeSelection(rows = row + 1, cols = col + 1)
}

internal fun tableSizeCenteredScrollTarget(
  index: Int,
  viewportExtent: Float,
  maxScroll: Int,
  cellSize: Float,
  gap: Float,
  outerPadding: Float,
): Int {
  val cellCenter = outerPadding + index * (cellSize + gap) + cellSize / 2f
  return (cellCenter - viewportExtent / 2f).roundToInt().coerceIn(0, maxScroll.coerceAtLeast(0))
}

@Composable
private fun TableSizeCell(selected: Boolean) {
  val shape = AppShapes.rounded(4.dp)
  Box(
    modifier =
      Modifier.size(TableSizeGridCellSize)
        .focusProperties { canFocus = false }
        .clip(shape)
        .background(
          color =
            if (selected) AppTheme.colors.palette.blue.copy(alpha = 0.25f)
            else AppTheme.colors.surfaceDefault,
          shape = shape,
        )
        .border(
          width = ToolbarTableSizeBorderWidth,
          color = if (selected) AppTheme.colors.palette.blue else AppTheme.colors.borderDefault,
          shape = shape,
        )
  )
}

@Composable
private fun TableInsertButton(
  rows: Int,
  cols: Int,
  onClick: () -> Unit,
  modifier: Modifier = Modifier,
) {
  val shape = AppShapes.rounded(999.dp)

  InteractionScope {
    val interactionSource =
      LocalInteractionSource.current ?: remember { MutableInteractionSource() }
    val pressed by interactionSource.collectIsPressedAsState()

    Row(
      modifier =
        modifier
          .focusProperties { canFocus = false }
          .clip(shape)
          .background(
            if (pressed) AppTheme.colors.surfaceInset else AppTheme.colors.surfaceDefault,
            shape,
          )
          .border(ToolbarTableSizeBorderWidth, AppTheme.colors.borderEmphasis, shape)
          .clickable(
            interactionSource = interactionSource,
            indication = null,
            role = Role.Button,
            onClickLabel = "${rows}×${cols} 표 삽입",
            onClick = onClick,
          )
          .pressScale(TableInsertButtonPressedScale)
          .height(ToolbarHeight)
          .padding(horizontal = 16.dp),
      horizontalArrangement = Arrangement.spacedBy(5.dp),
      verticalAlignment = Alignment.CenterVertically,
    ) {
      Icon(
        icon = Lucide.Table,
        contentDescription = null,
        modifier = Modifier.size(18.dp),
        tint = AppTheme.colors.textMuted,
      )
      Text(
        text = "${rows}×${cols} 삽입",
        style = AppTheme.typography.action,
        color = AppTheme.colors.textDefault,
      )
    }
  }
}

private const val MaxTableSize = 10
private const val DefaultTableRows = 3
private const val DefaultTableCols = 3
private val TableSizeGridCellSize = 36.dp
private val TableSizePanelPadding = TableSizeGridCellSize / 2f
private val TableSizeGridGap = 4.dp
private val TableSizeGridOuterPadding = TableSizeGridGap
private val TableSizeGridExtent =
  TableSizeGridCellSize * MaxTableSize.toFloat() +
    TableSizeGridGap * (MaxTableSize - 1).toFloat() +
    TableSizeGridOuterPadding * 2f
private val ToolbarTableSizeBorderWidth = 1.dp
private const val TableInsertButtonPressedScale = 0.96f
private const val TableSizeDragCenterAnimationMillis = 90
private const val TableSizeCenterAnimationMillis = 180
private const val TableSizeHoldResampleMillis = 90
