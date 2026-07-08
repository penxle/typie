package co.typie.screen.editor.editor.overlay

import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.offset
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.geometry.Rect
import androidx.compose.ui.geometry.Size
import androidx.compose.ui.semantics.Role
import androidx.compose.ui.semantics.contentDescription
import androidx.compose.ui.semantics.role
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.unit.Dp
import androidx.compose.ui.unit.IntOffset
import androidx.compose.ui.unit.dp
import co.typie.editor.Editor
import co.typie.editor.EditorViewportTransform
import co.typie.editor.ext.isCollapsed
import co.typie.editor.ffi.Axis
import co.typie.editor.ffi.Message
import co.typie.editor.ffi.NodeOp
import co.typie.editor.ffi.Selection
import co.typie.editor.ffi.TableOp
import co.typie.editor.ffi.TableOverlay
import co.typie.editor.runtime.EditorUiState
import co.typie.ext.clickable
import co.typie.icons.Lucide
import co.typie.screen.editor.editor.subpane.EditorTableAxisActionsTarget
import co.typie.ui.icon.Icon
import co.typie.ui.theme.AppShapes
import co.typie.ui.theme.AppTheme
import co.typie.ui.theme.shadow
import kotlin.math.max
import kotlin.math.min
import kotlin.math.roundToInt

private val TableAxisSelectorTableOverlap = 9.dp
private val TableAxisSelectorScreenPadding = 4.dp
private val TableAxisSelectorButtonShape = AppShapes.rounded(4.dp)
private val TableAxisSelectorColumnButtonWidth = 24.dp
private val TableAxisSelectorColumnButtonHeight = 18.dp
private val TableAxisSelectorRowButtonWidth = 18.dp
private val TableAxisSelectorRowButtonHeight = 24.dp

internal data class EditorTableAxisSelector(
  val overlay: TableOverlay,
  val axis: Axis,
  val index: Int,
  val count: Int,
  val backgroundColor: String?,
  val center: Offset,
)

@Composable
internal fun EditorTableAxisSelectionOverlay(
  editor: Editor,
  uiState: EditorUiState,
  editorRectInOverlay: Rect,
  overlaySize: Size,
  density: Float,
  onTableAxisActionsRequest: (EditorTableAxisActionsTarget, Selection?) -> Unit,
) {
  if (!uiState.focused || density <= 0f) {
    return
  }

  val placements =
    resolveTableAxisSelectorOverlayPlacements(
      editor = editor,
      uiState = uiState,
      editorRectInOverlay = editorRectInOverlay,
      overlaySize = overlaySize,
      density = density,
    )
  if (placements.isEmpty()) {
    return
  }

  Box(modifier = Modifier.fillMaxSize()) {
    placements.forEach { placement ->
      EditorTableAxisSelectorButton(
        placement = placement,
        onClick = {
          val selector = placement.selector
          editor.sendTableAxisOp(
            selector = selector,
            op = TableOp.SelectAxis(axis = selector.axis, index = selector.index),
          )
          onTableAxisActionsRequest(selector.toActionsTarget(), editor.selection)
        },
      )
    }
  }
}

internal fun resolveTableAxisSelectors(editor: Editor): List<EditorTableAxisSelector> {
  val selectionCollapsed = editor.selection.isCollapsed()
  return editor.tableOverlays.flatMap { overlay ->
    resolveTableAxisSelectors(overlay = overlay, selectionCollapsed = selectionCollapsed)
  }
}

internal fun resolveTableAxisSelectors(
  overlay: TableOverlay,
  selectionCollapsed: Boolean,
): List<EditorTableAxisSelector> {
  if (!overlay.isFocused) {
    return emptyList()
  }

  return buildList {
    resolveTableRowAxisSelector(overlay = overlay, selectionCollapsed = selectionCollapsed)?.let {
      add(it)
    }
    resolveTableColumnAxisSelector(overlay = overlay, selectionCollapsed = selectionCollapsed)
      ?.let { add(it) }
  }
}

private fun resolveTableRowAxisSelector(
  overlay: TableOverlay,
  selectionCollapsed: Boolean,
): EditorTableAxisSelector? {
  val rowIndex =
    overlay.cellSelection?.let { selection ->
      val rowStart = min(selection.anchorRow, selection.headRow)
      val rowEnd = max(selection.anchorRow, selection.headRow)
      rowStart.takeIf { rowStart == rowEnd }
    }
      ?: overlay.focusedRowIndex
        ?.takeIf { overlay.cellSelection == null && selectionCollapsed }
        ?.let { localRow -> overlay.rows.getOrNull(localRow)?.index }
      ?: return null

  val localRow = overlay.rows.indexOfFirst { row -> row.index == rowIndex }
  if (localRow < 0) {
    return null
  }

  val top = if (localRow == 0) 0f else overlay.rows[localRow - 1].position
  val bottom = overlay.rows[localRow].position
  return EditorTableAxisSelector(
    overlay = overlay,
    axis = Axis.Horizontal,
    index = rowIndex,
    count = overlay.rowCount,
    backgroundColor = overlay.rows[localRow].backgroundColor,
    center = Offset(x = overlay.bounds.x, y = overlay.bounds.y + (top + bottom) / 2f),
  )
}

private fun resolveTableColumnAxisSelector(
  overlay: TableOverlay,
  selectionCollapsed: Boolean,
): EditorTableAxisSelector? {
  val colIndex =
    overlay.cellSelection?.let { selection ->
      val colStart = min(selection.anchorCol, selection.headCol)
      val colEnd = max(selection.anchorCol, selection.headCol)
      colStart.takeIf { colStart == colEnd }
    }
      ?: overlay.focusedColIndex?.takeIf { overlay.cellSelection == null && selectionCollapsed }
      ?: return null

  val localCol = overlay.columns.indexOfFirst { column -> column.index == colIndex }
  if (localCol < 0) {
    return null
  }

  val left = if (localCol == 0) 0f else overlay.columns[localCol - 1].position
  val right = overlay.columns[localCol].position
  return EditorTableAxisSelector(
    overlay = overlay,
    axis = Axis.Vertical,
    index = colIndex,
    count = overlay.columns.size,
    backgroundColor = overlay.columns[localCol].backgroundColor,
    center = Offset(x = overlay.bounds.x + (left + right) / 2f, y = overlay.bounds.y),
  )
}

internal fun resolveTableAxisSelectorOverlayPlacements(
  editor: Editor,
  uiState: EditorUiState,
  editorRectInOverlay: Rect,
  overlaySize: Size,
  density: Float,
): List<EditorTableAxisSelectorOverlayPlacement> {
  if (density <= 0f) {
    return emptyList()
  }

  val transform = uiState.resolveViewportTransform(pageSizes = editor.pageSizes)
  return resolveTableAxisSelectors(editor).mapNotNull { selector ->
    val center =
      resolvePositionInOverlay(
        selector = selector,
        position = selector.center,
        transform = transform,
        editorRectInOverlay = editorRectInOverlay,
        density = density,
      ) ?: return@mapNotNull null
    val (width, height) =
      when (selector.axis) {
        Axis.Horizontal -> TableAxisSelectorRowButtonWidth to TableAxisSelectorRowButtonHeight
        Axis.Vertical -> TableAxisSelectorColumnButtonWidth to TableAxisSelectorColumnButtonHeight
      }
    val buttonRect =
      resolveSelectorButtonRect(
        center = center,
        axis = selector.axis,
        widthPx = width.value * density,
        heightPx = height.value * density,
        tableOverlapPx = TableAxisSelectorTableOverlap.value * density,
        screenPaddingPx = TableAxisSelectorScreenPadding.value * density,
        overlaySize = overlaySize,
      )
    EditorTableAxisSelectorOverlayPlacement(
      selector = selector,
      buttonRect = buttonRect,
      width = width,
      height = height,
    )
  }
}

internal data class EditorTableAxisSelectorOverlayPlacement(
  val selector: EditorTableAxisSelector,
  val buttonRect: Rect,
  val width: Dp,
  val height: Dp,
)

private fun Editor.sendTableAxisOp(selector: EditorTableAxisSelector, op: TableOp) {
  sync { enqueue(selector.tableMessage(op)) }
}

private fun EditorTableAxisSelector.tableMessage(op: TableOp): Message =
  Message.Node(NodeOp.Table(id = overlay.tableId, op = op))

private fun EditorTableAxisSelector.toActionsTarget(): EditorTableAxisActionsTarget =
  EditorTableAxisActionsTarget(
    tableId = overlay.tableId,
    axis = axis,
    index = index,
    count = count,
    backgroundColor = backgroundColor,
  )

internal fun resolveSelectorButtonRect(
  center: Offset,
  axis: Axis,
  widthPx: Float,
  heightPx: Float,
  tableOverlapPx: Float,
  screenPaddingPx: Float,
  overlaySize: Size,
): Rect {
  val rawLeft =
    when (axis) {
      Axis.Horizontal -> center.x - widthPx + tableOverlapPx
      Axis.Vertical -> center.x - widthPx / 2f
    }
  val rawTop =
    when (axis) {
      Axis.Horizontal -> center.y - heightPx / 2f
      Axis.Vertical -> center.y - heightPx + tableOverlapPx
    }
  val maxLeft = (overlaySize.width - widthPx - screenPaddingPx).coerceAtLeast(screenPaddingPx)
  val maxTop = (overlaySize.height - heightPx - screenPaddingPx).coerceAtLeast(screenPaddingPx)
  val left = rawLeft.coerceIn(screenPaddingPx, maxLeft)
  val top = rawTop.coerceIn(screenPaddingPx, maxTop)
  return Rect(left = left, top = top, right = left + widthPx, bottom = top + heightPx)
}

private fun resolvePositionInOverlay(
  selector: EditorTableAxisSelector,
  position: Offset,
  transform: EditorViewportTransform,
  editorRectInOverlay: Rect,
  density: Float,
): Offset? {
  val global =
    transform.localToGlobal(page = selector.overlay.pageIdx, x = position.x, y = position.y)
      ?: return null
  return Offset(
    x = editorRectInOverlay.left + global.x * density,
    y = editorRectInOverlay.top + global.y * density,
  )
}

@Composable
private fun EditorTableAxisSelectorButton(
  placement: EditorTableAxisSelectorOverlayPlacement,
  onClick: () -> Unit,
) {
  val icon =
    when (placement.selector.axis) {
      Axis.Horizontal -> Lucide.EllipsisVertical
      Axis.Vertical -> Lucide.Ellipsis
    }
  val description =
    when (placement.selector.axis) {
      Axis.Horizontal -> "행 메뉴"
      Axis.Vertical -> "열 메뉴"
    }
  Box(
    modifier =
      Modifier.offset {
          IntOffset(
            x = placement.buttonRect.left.roundToInt(),
            y = placement.buttonRect.top.roundToInt(),
          )
        }
        .width(placement.width)
        .height(placement.height)
        .shadow(AppTheme.shadows.sm, TableAxisSelectorButtonShape)
        .clip(TableAxisSelectorButtonShape)
        .border(1.dp, AppTheme.colors.borderEmphasis, TableAxisSelectorButtonShape)
        .background(AppTheme.colors.surfaceDefault, TableAxisSelectorButtonShape)
        .semantics {
          contentDescription = description
          role = Role.Button
        }
        .clickable { onClick() },
    contentAlignment = Alignment.Center,
  ) {
    Icon(
      icon = icon,
      contentDescription = null,
      modifier = Modifier.size(14.dp),
      tint = AppTheme.colors.textMuted,
    )
  }
}
