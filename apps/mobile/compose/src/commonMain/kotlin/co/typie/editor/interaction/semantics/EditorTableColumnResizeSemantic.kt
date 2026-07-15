package co.typie.editor.interaction.semantics

import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.setValue
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.geometry.Rect
import co.typie.editor.Editor
import co.typie.editor.ffi.Message
import co.typie.editor.ffi.NodeOp
import co.typie.editor.ffi.TableOp
import co.typie.editor.ffi.TableOverlay
import co.typie.editor.interaction.EditorInteractionGeometry
import co.typie.editor.interaction.EditorTableCellSelection
import co.typie.editor.interaction.EditorTableCellSelectionHandleTouchTargetDp
import co.typie.editor.interaction.resolveActiveTableCellSelection
import kotlin.math.abs
import kotlin.math.max
import kotlin.math.roundToInt

private const val TableColumnResizeTouchWidthDp = 24f
private const val TableColumnResizeMinColumnWidth = 40f
private const val TableColumnResizeBorderWidth = 1f
private const val TableResizeLimitEpsilon = 0.5f
private const val TableResizeCommitEpsilon = 0.01f

internal data class EditorTableColumnResizeTarget(
  val overlay: TableOverlay,
  val colIndex: Int,
  val localColIndex: Int,
  val isTableResize: Boolean,
  val pageX: Float,
)

internal data class EditorTableColumnResizePlacement(
  val target: EditorTableColumnResizeTarget,
  val centerX: Float,
  val top: Float,
  val bottom: Float,
  val handleRects: List<Rect>,
  val pxPerPageUnit: Float,
)

internal data class EditorTableColumnResizeDraft(
  val tableId: String,
  val colIndex: Int,
  val isTableResize: Boolean,
  val baseCenterX: Float,
  val top: Float,
  val bottom: Float,
  val initialWidths: List<Float>,
  val initialTableWidth: Float,
  val contentWidth: Float,
  val minProportionWidth: Float,
  val maxProportionWidth: Float,
  val deltaX: Float,
  val pxPerPageUnit: Float,
)

internal data class EditorTableColumnResizePresentation(
  val pressed: Boolean,
  val draft: EditorTableColumnResizeDraft?,
)

internal class EditorTableColumnResizeSemantic {
  private var editor: Editor? = null
  private var placement: EditorTableColumnResizePlacement? = null
  private var draft by mutableStateOf<EditorTableColumnResizeDraft?>(null)
  private var pressed by mutableStateOf(false)

  val presentation: EditorTableColumnResizePresentation
    get() = EditorTableColumnResizePresentation(pressed = pressed, draft = draft)

  fun press(editor: Editor, placement: EditorTableColumnResizePlacement) {
    this.editor = editor
    this.placement = placement
    pressed = true
  }

  fun start(): Boolean {
    val currentPlacement = placement ?: return false
    draft = currentPlacement.toDraft()
    return true
  }

  fun update(deltaPx: Float) {
    draft = draft?.let { current ->
      current.copy(deltaX = current.deltaX + dragDeltaToPageDelta(deltaPx, current.pxPerPageUnit))
    }
  }

  fun end() {
    val currentEditor = editor
    val finished = draft
    clear()
    if (currentEditor != null && finished != null) {
      currentEditor.commitTableResize(finished)
    }
  }

  fun cancel() {
    clear()
  }

  fun reset() {
    clear()
  }

  private fun clear() {
    editor = null
    placement = null
    draft = null
    pressed = false
  }
}

internal fun resolveTableColumnResizeTarget(
  selection: EditorTableCellSelection
): EditorTableColumnResizeTarget? {
  val overlay = selection.overlay
  if (!overlay.isFocused || overlay.columns.isEmpty()) {
    return null
  }
  val colIndex = selection.range.colEnd
  val localColIndex = overlay.columns.indexOfFirst { column -> column.index == colIndex }
  if (localColIndex < 0) {
    return null
  }
  val columnRight = overlay.columns[localColIndex].position
  return EditorTableColumnResizeTarget(
    overlay = overlay,
    colIndex = colIndex,
    localColIndex = localColIndex,
    isTableResize = localColIndex == overlay.columns.lastIndex,
    pageX = overlay.bounds.x + columnRight,
  )
}

internal fun resolveTableColumnResizePlacement(
  editor: Editor,
  geometry: EditorInteractionGeometry,
): EditorTableColumnResizePlacement? {
  val density = geometry.density
  if (density <= 0f) {
    return null
  }
  val selection = resolveActiveTableCellSelection(editor) ?: return null
  val target = resolveTableColumnResizeTarget(selection) ?: return null
  val overlay = target.overlay
  val topCenter =
    geometry.resolvePagePosition(page = overlay.pageIdx, x = target.pageX, y = overlay.bounds.y)
      ?: return null
  val bottomCenter =
    geometry.resolvePagePosition(
      page = overlay.pageIdx,
      x = target.pageX,
      y = overlay.bounds.y + overlay.bounds.height,
    ) ?: return null
  val blockedCenter =
    selection.geometry.handleCenter?.let { center ->
      geometry.resolvePagePosition(page = overlay.pageIdx, x = center.x, y = center.y)
    }
  val halfWidth = TableColumnResizeTouchWidthDp * density / 2f
  val blockedHalfSize = EditorTableCellSelectionHandleTouchTargetDp * density / 2f
  val handleRects =
    splitTableColumnResizeHitRects(
      centerX = topCenter.x,
      top = topCenter.y,
      bottom = bottomCenter.y,
      halfWidth = halfWidth,
      blockedCenter = blockedCenter,
      blockedHalfSize = blockedHalfSize,
    )
  if (handleRects.isEmpty()) {
    return null
  }
  val pxPerPageUnit =
    if (overlay.bounds.height > 0f) {
      abs(bottomCenter.y - topCenter.y) / overlay.bounds.height
    } else {
      density
    }
  return EditorTableColumnResizePlacement(
    target = target,
    centerX = topCenter.x,
    top = topCenter.y,
    bottom = bottomCenter.y,
    handleRects = handleRects,
    pxPerPageUnit = pxPerPageUnit,
  )
}

internal fun splitTableColumnResizeHitRects(
  centerX: Float,
  top: Float,
  bottom: Float,
  halfWidth: Float,
  blockedCenter: Offset?,
  blockedHalfSize: Float,
): List<Rect> {
  if (bottom <= top || halfWidth <= 0f) {
    return emptyList()
  }
  val full =
    Rect(left = centerX - halfWidth, top = top, right = centerX + halfWidth, bottom = bottom)
  if (
    blockedCenter == null ||
      blockedHalfSize <= 0f ||
      blockedCenter.x < full.left ||
      blockedCenter.x > full.right
  ) {
    return listOf(full)
  }

  val blockedTop = (blockedCenter.y - blockedHalfSize).coerceIn(top, bottom)
  val blockedBottom = (blockedCenter.y + blockedHalfSize).coerceIn(top, bottom)
  if (blockedBottom <= top || blockedTop >= bottom || blockedTop >= blockedBottom) {
    return listOf(full)
  }

  return buildList {
    if (blockedTop > top) {
      add(full.copy(bottom = blockedTop))
    }
    if (blockedBottom < bottom) {
      add(full.copy(top = blockedBottom))
    }
  }
}

internal fun resizeTableColumnWidths(
  widths: List<Float>,
  colIndex: Int,
  deltaX: Float,
): List<Float> {
  if (widths.isEmpty() || colIndex < 0 || colIndex >= widths.lastIndex) {
    return widths
  }
  val clampedDelta =
    clampTableColumnResizeDelta(widths = widths, colIndex = colIndex, deltaX = deltaX)
  return widths.mapIndexed { index, width ->
    when (index) {
      colIndex -> width + clampedDelta
      colIndex + 1 -> width - clampedDelta
      else -> width
    }
  }
}

internal fun resolveTableResizeProportion(overlay: TableOverlay, deltaX: Float): Int? =
  resolveTableResizeProportion(
    colCount = overlay.columns.size,
    currentTableWidth = overlay.bounds.width,
    contentWidth = overlay.contentWidth,
    minProportionWidth = overlay.minProportionWidth,
    maxProportionWidth = overlay.maxProportionWidth,
    deltaX = deltaX,
  )

internal fun resolveTableResizeCommittedDelta(overlay: TableOverlay, deltaX: Float): Float? =
  resolveTableResizeCommittedDelta(
    colCount = overlay.columns.size,
    currentTableWidth = overlay.bounds.width,
    contentWidth = overlay.contentWidth,
    minProportionWidth = overlay.minProportionWidth,
    maxProportionWidth = overlay.maxProportionWidth,
    deltaX = deltaX,
  )

internal fun resolveTableResizePreviewDelta(overlay: TableOverlay, deltaX: Float): Float =
  resolveTableResizePreviewDelta(
    colCount = overlay.columns.size,
    currentTableWidth = overlay.bounds.width,
    contentWidth = overlay.contentWidth,
    minProportionWidth = overlay.minProportionWidth,
    maxProportionWidth = overlay.maxProportionWidth,
    deltaX = deltaX,
  )

internal fun dragDeltaToPageDelta(deltaPx: Float, pxPerPageUnit: Float): Float =
  if (pxPerPageUnit.isFinite() && pxPerPageUnit > 0f) deltaPx / pxPerPageUnit else 0f

private fun resolveTableResizeProportion(
  colCount: Int,
  currentTableWidth: Float,
  contentWidth: Float,
  minProportionWidth: Float,
  maxProportionWidth: Float,
  deltaX: Float,
): Int? {
  if (colCount <= 0 || contentWidth <= 0f) {
    return null
  }
  val clampedDelta =
    clampTableResizeDelta(
      colCount = colCount,
      currentTableWidth = currentTableWidth,
      contentWidth = contentWidth,
      minProportionWidth = minProportionWidth,
      maxProportionWidth = maxProportionWidth,
      deltaX = deltaX,
    )
  val nextWidth = currentTableWidth + clampedDelta
  return ((nextWidth / contentWidth) * 100f).roundToInt().coerceAtLeast(0)
}

private fun resolveTableResizeCommittedDelta(
  colCount: Int,
  currentTableWidth: Float,
  contentWidth: Float,
  minProportionWidth: Float,
  maxProportionWidth: Float,
  deltaX: Float,
): Float? {
  if (contentWidth <= 0f) {
    return null
  }
  val proportion =
    resolveTableResizeProportion(
      colCount = colCount,
      currentTableWidth = currentTableWidth,
      contentWidth = contentWidth,
      minProportionWidth = minProportionWidth,
      maxProportionWidth = maxProportionWidth,
      deltaX = deltaX,
    ) ?: return null
  val initialProportion = ((currentTableWidth / contentWidth) * 100f).roundToInt().coerceAtLeast(0)
  if (proportion == initialProportion) {
    return 0f
  }
  return contentWidth * (proportion / 100f) - currentTableWidth
}

private fun resolveTableResizePreviewDelta(
  colCount: Int,
  currentTableWidth: Float,
  contentWidth: Float,
  minProportionWidth: Float,
  maxProportionWidth: Float,
  deltaX: Float,
): Float =
  clampTableResizeDelta(
    colCount = colCount,
    currentTableWidth = currentTableWidth,
    contentWidth = contentWidth,
    minProportionWidth = minProportionWidth,
    maxProportionWidth = maxProportionWidth,
    deltaX = deltaX,
  )

private fun clampTableColumnResizeDelta(widths: List<Float>, colIndex: Int, deltaX: Float): Float {
  if (widths.isEmpty() || colIndex < 0 || colIndex >= widths.lastIndex) {
    return 0f
  }
  val minDelta = TableColumnResizeMinColumnWidth - widths[colIndex]
  val maxDelta = widths[colIndex + 1] - TableColumnResizeMinColumnWidth
  return if (minDelta <= maxDelta) deltaX.coerceIn(minDelta, maxDelta) else 0f
}

private fun clampTableResizeDelta(
  colCount: Int,
  currentTableWidth: Float,
  contentWidth: Float,
  minProportionWidth: Float,
  maxProportionWidth: Float,
  deltaX: Float,
): Float {
  if (colCount <= 0 || contentWidth <= 0f) {
    return 0f
  }
  val minTableWidth =
    max(minTableWidthForColumns(colCount), minProportionWidth.takeIf { it.isFinite() } ?: 0f)
  val maxTableWidth =
    max(minTableWidth, maxProportionWidth.takeIf { it.isFinite() && it > 0f } ?: contentWidth)
  val effectiveMinTableWidth =
    if (currentTableWidth <= minTableWidth + TableResizeLimitEpsilon) currentTableWidth
    else minTableWidth
  val minDelta = effectiveMinTableWidth - currentTableWidth
  val maxDelta = maxTableWidth - currentTableWidth
  return if (minDelta <= maxDelta) deltaX.coerceIn(minDelta, maxDelta) else 0f
}

private fun minTableWidthForColumns(colCount: Int): Float =
  if (colCount <= 0) 0f
  else TableColumnResizeMinColumnWidth * colCount + TableColumnResizeBorderWidth * (colCount + 1)

private fun toRatioWidths(widths: List<Float>): List<Float> {
  if (widths.isEmpty()) {
    return emptyList()
  }
  val safe = widths.map { width -> if (width.isFinite() && width > 0f) width else 0f }
  val total = safe.sum()
  return if (total <= 0f) List(widths.size) { 1f / widths.size }
  else safe.map { width -> width / total }
}

private fun hasWidthChange(before: List<Float>, after: List<Float>): Boolean =
  before.size != after.size ||
    before.indices.any { index -> abs(before[index] - after[index]) > TableResizeCommitEpsilon }

internal fun resolveTableColumnResizePreviewDelta(draft: EditorTableColumnResizeDraft): Float =
  if (draft.isTableResize) {
    resolveTableResizePreviewDelta(
      colCount = draft.initialWidths.size,
      currentTableWidth = draft.initialTableWidth,
      contentWidth = draft.contentWidth,
      minProportionWidth = draft.minProportionWidth,
      maxProportionWidth = draft.maxProportionWidth,
      deltaX = draft.deltaX,
    )
  } else {
    clampTableColumnResizeDelta(
      widths = draft.initialWidths,
      colIndex = draft.colIndex,
      deltaX = draft.deltaX,
    )
  }

private fun EditorTableColumnResizePlacement.toDraft(): EditorTableColumnResizeDraft {
  val overlay = target.overlay
  return EditorTableColumnResizeDraft(
    tableId = overlay.tableId,
    colIndex = target.localColIndex,
    isTableResize = target.isTableResize,
    baseCenterX = centerX,
    top = top,
    bottom = bottom,
    initialWidths = overlay.columns.map { column -> column.widthAsPx },
    initialTableWidth = overlay.bounds.width,
    contentWidth = overlay.contentWidth,
    minProportionWidth = overlay.minProportionWidth,
    maxProportionWidth = overlay.maxProportionWidth,
    deltaX = 0f,
    pxPerPageUnit = pxPerPageUnit,
  )
}

private fun Editor.commitTableResize(draft: EditorTableColumnResizeDraft) {
  val op =
    if (draft.isTableResize) {
      val proportion =
        resolveTableResizeProportion(
          colCount = draft.initialWidths.size,
          currentTableWidth = draft.initialTableWidth,
          contentWidth = draft.contentWidth,
          minProportionWidth = draft.minProportionWidth,
          maxProportionWidth = draft.maxProportionWidth,
          deltaX = draft.deltaX,
        ) ?: return
      val initialProportion =
        ((draft.initialTableWidth / draft.contentWidth) * 100f).roundToInt().coerceAtLeast(0)
      if (proportion == initialProportion) {
        return
      }
      TableOp.SetProportion(proportion = proportion)
    } else {
      val nextWidths =
        resizeTableColumnWidths(
          widths = draft.initialWidths,
          colIndex = draft.colIndex,
          deltaX = draft.deltaX,
        )
      if (!hasWidthChange(draft.initialWidths, nextWidths)) {
        return
      }
      TableOp.SetColumnWidths(widths = toRatioWidths(nextWidths))
    }

  sync { enqueue(Message.Node(NodeOp.Table(id = draft.tableId, op = op))) }
}
