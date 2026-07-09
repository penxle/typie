package co.typie.screen.editor.editor.overlay

import androidx.compose.foundation.Canvas
import androidx.compose.foundation.gestures.awaitEachGesture
import androidx.compose.foundation.gestures.awaitFirstDown
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.offset
import androidx.compose.foundation.layout.width
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberUpdatedState
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.geometry.CornerRadius
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.geometry.Rect
import androidx.compose.ui.geometry.Size
import androidx.compose.ui.input.pointer.PointerEventPass
import androidx.compose.ui.input.pointer.pointerInput
import androidx.compose.ui.platform.LocalViewConfiguration
import androidx.compose.ui.unit.IntOffset
import androidx.compose.ui.unit.dp
import co.typie.editor.Editor
import co.typie.editor.EditorViewportTransform
import co.typie.editor.ffi.Message
import co.typie.editor.ffi.NodeOp
import co.typie.editor.ffi.TableOp
import co.typie.editor.ffi.TableOverlay
import co.typie.editor.interaction.EditorInteractionController
import co.typie.editor.interaction.EditorInteractionScope
import co.typie.editor.interaction.EditorTableCellSelection
import co.typie.editor.interaction.EditorTableCellSelectionHandleTouchTargetDp
import co.typie.editor.interaction.LocalEditorInteractionScope
import co.typie.editor.interaction.editorInteractions
import co.typie.editor.interaction.resolveActiveTableCellSelection
import co.typie.editor.runtime.EditorUiState
import co.typie.editor.viewport.EditorViewportState
import co.typie.ui.theme.AppTheme
import kotlin.math.abs
import kotlin.math.max
import kotlin.math.min
import kotlin.math.roundToInt

private const val TableColumnResizeTouchWidthDp = 24f
private const val TableColumnResizeDragSlopDp = 8f
private const val TableColumnResizeVisualWidthDp = 3f
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

internal data class EditorTableColumnResizeOverlayPlacement(
  val target: EditorTableColumnResizeTarget,
  val centerX: Float,
  val top: Float,
  val bottom: Float,
  val handleRects: List<Rect>,
  val pxPerPageUnit: Float,
)

private data class EditorTableColumnResizeDraft(
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
) {
  fun copyWithDelta(deltaX: Float): EditorTableColumnResizeDraft = copy(deltaX = deltaX)
}

@Composable
internal fun EditorTableColumnResizeOverlay(
  editor: Editor,
  uiState: EditorUiState,
  editorRectInOverlay: Rect,
  density: Float,
  viewportState: EditorViewportState,
) {
  if (!uiState.focused || density <= 0f) {
    return
  }

  val placement =
    resolveTableColumnResizeOverlayPlacement(
      editor = editor,
      uiState = uiState,
      editorRectInOverlay = editorRectInOverlay,
      density = density,
    ) ?: return
  var draft by remember { mutableStateOf<EditorTableColumnResizeDraft?>(null) }
  var resizeHandlePressed by remember { mutableStateOf(false) }
  val currentEditor by rememberUpdatedState(editor)
  val interactionScope = LocalEditorInteractionScope.current
  val interactionController = interactionScope.controller
  val color = AppTheme.colors.palette.blue
  val activeDraft = draft
  val resizeHandleActive = activeDraft != null || resizeHandlePressed
  val visualCenterX =
    activeDraft?.let { it.baseCenterX + resolveResizeDelta(it) * it.pxPerPageUnit }
      ?: placement.centerX
  val visualTop = activeDraft?.top ?: placement.top
  val visualBottom = activeDraft?.bottom ?: placement.bottom
  val visualWidth = TableColumnResizeVisualWidthDp * density
  val verticalInset = 2f * density

  Box(modifier = Modifier.fillMaxSize()) {
    Canvas(modifier = Modifier.fillMaxSize()) {
      val height = (visualBottom - visualTop - verticalInset * 2f).coerceAtLeast(0f)
      if (height > 0f) {
        drawRoundRect(
          color = color.copy(alpha = if (resizeHandleActive) 0.85f else 0.35f),
          topLeft = Offset(x = visualCenterX - visualWidth / 2f, y = visualTop + verticalInset),
          size = Size(width = visualWidth, height = height),
          cornerRadius = CornerRadius(visualWidth / 2f, visualWidth / 2f),
        )
      }
    }

    placement.handleRects.forEach { rect ->
      TableColumnResizeHandle(
        rect = rect,
        density = density,
        editorRectInOverlay = editorRectInOverlay,
        viewportState = viewportState,
        interactionScope = interactionScope,
        interactionController = interactionController,
        onPressedChange = { resizeHandlePressed = it },
        onStart = {
          uiState.contextMenu.hide()
          draft = placement.toDraft()
        },
        onDrag = { deltaX ->
          draft = draft?.let {
            it.copyWithDelta(it.deltaX + dragDeltaToPageDelta(deltaX, it.pxPerPageUnit))
          }
        },
        onEnd = {
          val finished = draft
          draft = null
          finished?.let { currentEditor.commitTableResize(it) }
        },
      )
    }
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

internal fun resolveTableColumnResizeOverlayPlacement(
  editor: Editor,
  uiState: EditorUiState,
  editorRectInOverlay: Rect,
  density: Float,
): EditorTableColumnResizeOverlayPlacement? {
  if (density <= 0f) {
    return null
  }
  val selection = resolveActiveTableCellSelection(editor) ?: return null
  val target = resolveTableColumnResizeTarget(selection) ?: return null
  val overlay = target.overlay
  val transform = uiState.resolveViewportTransform(pageSizes = editor.pageSizes)
  val pxPerPageUnit = density * normalizedDisplayZoom(uiState.displayZoom)
  val topCenter =
    resolvePositionInOverlay(
      page = overlay.pageIdx,
      x = target.pageX,
      y = overlay.bounds.y,
      transform = transform,
      editorRectInOverlay = editorRectInOverlay,
      density = density,
    ) ?: return null
  val bottomCenter =
    resolvePositionInOverlay(
      page = overlay.pageIdx,
      x = target.pageX,
      y = overlay.bounds.y + overlay.bounds.height,
      transform = transform,
      editorRectInOverlay = editorRectInOverlay,
      density = density,
    ) ?: return null
  val blockedCenter =
    selection.geometry.handleCenter?.let { center ->
      resolvePositionInOverlay(
        page = overlay.pageIdx,
        x = center.x,
        y = center.y,
        transform = transform,
        editorRectInOverlay = editorRectInOverlay,
        density = density,
      )
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
  return EditorTableColumnResizeOverlayPlacement(
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
  if (pxPerPageUnit.isFinite() && pxPerPageUnit > 0f) {
    deltaPx / pxPerPageUnit
  } else {
    0f
  }

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
  if (minDelta > maxDelta) {
    return 0f
  }
  return deltaX.coerceIn(minDelta, maxDelta)
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
    if (currentTableWidth <= minTableWidth + TableResizeLimitEpsilon) {
      currentTableWidth
    } else {
      minTableWidth
    }
  val minDelta = effectiveMinTableWidth - currentTableWidth
  val maxDelta = maxTableWidth - currentTableWidth
  if (minDelta > maxDelta) {
    return 0f
  }
  return deltaX.coerceIn(minDelta, maxDelta)
}

private fun minTableWidthForColumns(colCount: Int): Float =
  if (colCount <= 0) {
    0f
  } else {
    TableColumnResizeMinColumnWidth * colCount + TableColumnResizeBorderWidth * (colCount + 1)
  }

private fun toRatioWidths(widths: List<Float>): List<Float> {
  if (widths.isEmpty()) {
    return emptyList()
  }
  val safe = widths.map { width -> if (width.isFinite() && width > 0f) width else 0f }
  val total = safe.sum()
  if (total <= 0f) {
    return List(widths.size) { 1f / widths.size }
  }
  return safe.map { width -> width / total }
}

private fun hasWidthChange(before: List<Float>, after: List<Float>): Boolean {
  if (before.size != after.size) {
    return true
  }
  return before.indices.any { index ->
    abs(before[index] - after[index]) > TableResizeCommitEpsilon
  }
}

private fun resolveResizeDelta(draft: EditorTableColumnResizeDraft): Float =
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

private fun EditorTableColumnResizeOverlayPlacement.toDraft(): EditorTableColumnResizeDraft {
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

@Composable
private fun TableColumnResizeHandle(
  rect: Rect,
  density: Float,
  editorRectInOverlay: Rect,
  viewportState: EditorViewportState,
  interactionScope: EditorInteractionScope,
  interactionController: EditorInteractionController,
  onPressedChange: (Boolean) -> Unit,
  onStart: () -> Unit,
  onDrag: (Float) -> Unit,
  onEnd: () -> Unit,
) {
  val viewConfiguration = LocalViewConfiguration.current
  val currentOnStart by rememberUpdatedState(onStart)
  val currentOnDrag by rememberUpdatedState(onDrag)
  val currentOnEnd by rememberUpdatedState(onEnd)
  val currentOnPressedChange by rememberUpdatedState(onPressedChange)
  val dragSlop = min(viewConfiguration.touchSlop, TableColumnResizeDragSlopDp * density)
  Box(
    modifier =
      Modifier.offset { IntOffset(x = rect.left.roundToInt(), y = rect.top.roundToInt()) }
        .width((rect.width / density).dp)
        .height((rect.height / density).dp)
        .pointerInput(interactionController) {
          awaitPointerEventScope {
            while (true) {
              val event = awaitPointerEvent(PointerEventPass.Initial)
              if (event.changes.any { it.pressed && !it.previousPressed }) {
                interactionController.clearTapHistory()
              }
            }
          }
        }
        .pointerInput(Unit) {
          awaitEachGesture {
            val down = awaitFirstDown(requireUnconsumed = false, pass = PointerEventPass.Final)
            val pointerId = down.id
            val start = down.position
            var previous = start
            var dragging = false
            currentOnPressedChange(true)
            interactionScope.cancelTapDispatch()
            interactionScope.cancelLongPressDispatch()
            try {
              while (true) {
                val event = awaitPointerEvent()
                val change = event.changes.firstOrNull { it.id == pointerId } ?: break
                if (!change.pressed) {
                  break
                }
                val position = change.position
                if (!dragging) {
                  if ((position - start).getDistance() <= dragSlop) {
                    continue
                  }
                  dragging = true
                  currentOnStart()
                }
                currentOnDrag(position.x - previous.x)
                previous = position
                change.consume()
              }
            } finally {
              if (dragging) {
                currentOnEnd()
              }
              currentOnPressedChange(false)
            }
          }
        }
        .editorOverlayViewportWheelInput(
          viewportState = viewportState,
          interactionScope = interactionScope,
          targetRectInOverlay = rect,
        )
        .editorInteractions(
          density = density,
          interactionController = interactionController,
          coordinateResolver =
            EditorOverlayPointerTargetCoordinateResolver(
              editorRectInOverlay = editorRectInOverlay,
              targetRectInOverlay = rect,
            ),
        )
  ) {}
}

private fun resolvePositionInOverlay(
  page: Int,
  x: Float,
  y: Float,
  transform: EditorViewportTransform,
  editorRectInOverlay: Rect,
  density: Float,
): Offset? {
  val global = transform.localToGlobal(page = page, x = x, y = y) ?: return null
  return Offset(
    x = editorRectInOverlay.left + global.x * density,
    y = editorRectInOverlay.top + global.y * density,
  )
}

private fun normalizedDisplayZoom(displayZoom: Float): Float =
  if (displayZoom.isFinite() && displayZoom > 0f) {
    displayZoom
  } else {
    1f
  }
