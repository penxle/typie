package co.typie.editor.interaction.semantics

import androidx.compose.ui.geometry.Offset
import co.typie.editor.PagePoint
import co.typie.editor.ffi.Position
import co.typie.editor.ffi.Selection
import co.typie.editor.interaction.EditorEdgeAutoScrollViewport
import co.typie.editor.interaction.EditorGestureContext
import co.typie.ext.computeEdgeAutoScrollPlan
import kotlin.math.abs
import kotlin.math.sign
import kotlinx.coroutines.delay

private const val EditorEdgeAutoScrollTickMillis = 16L
private const val EditorEdgeAutoScrollThresholdDp = 30f
private const val EditorEdgeAutoScrollMinSpeedDpPerSecond = 250f
private const val EditorEdgeAutoScrollMaxSpeedDpPerSecond = 1000f

internal class EditorEdgeAutoScrollSemantic {
  private var activeRequest: EditorEdgeAutoScrollRequest? = null
  private var running = false
  private var lastDispatchedPoint: PagePoint? = null

  fun trackSelectionHandle(
    edgePosition: Offset,
    dispatchPosition: Offset,
    anchor: Position,
    baseSelection: Selection? = null,
    context: EditorGestureContext,
  ) {
    track(
      edgePosition = edgePosition,
      dispatchPosition = dispatchPosition,
      context = context,
      dispatch = { scrolled ->
        context.editor.dispatchSelectionHandleExtension(
          point = scrolled.point,
          anchor = anchor,
          baseSelection = baseSelection,
        )
      },
    )
  }

  fun trackSelectionHandle(
    edgePosition: Offset,
    dispatchPosition: Offset,
    context: EditorGestureContext,
    dispatch: (EditorEdgeAutoScrollDispatch) -> Boolean,
  ) {
    track(
      edgePosition = edgePosition,
      dispatchPosition = dispatchPosition,
      context = context,
      dispatch = dispatch,
    )
  }

  fun trackSelectionExpansion(
    edgePosition: Offset,
    dispatchPosition: Offset,
    context: EditorGestureContext,
  ) {
    track(
      edgePosition = edgePosition,
      dispatchPosition = dispatchPosition,
      context = context,
      dispatch = { scrolled ->
        val selectionContext = context.semantics.selectionExpansion.context(context.editor)
        selectionContext != null &&
          context.editor.dispatchSelectionExtension(
            point = scrolled.point,
            context = selectionContext,
          )
      },
    )
  }

  fun trackCursorMove(
    edgePosition: Offset,
    dispatchPosition: Offset,
    context: EditorGestureContext,
  ) {
    track(
      edgePosition = edgePosition,
      dispatchPosition = dispatchPosition,
      context = context,
      dispatch = { scrolled ->
        context.semantics.cursorMove.enqueuePrimaryClick(
          editor = context.editor,
          point = scrolled.point,
          clickCount = 1,
        )
      },
    )
  }

  fun track(edgePosition: Offset, context: EditorGestureContext, onScroll: (Offset) -> Unit) {
    trackWithResult(
      edgePosition = edgePosition,
      context = context,
      onScroll = { result -> onScroll(result.consumedDelta) },
    )
  }

  private fun trackWithResult(
    edgePosition: Offset,
    context: EditorGestureContext,
    onScroll: (EditorEdgeAutoScrollResult) -> Unit,
  ) {
    val viewport = context.geometry.resolveEdgeAutoScrollViewport()
    if (viewport == null || planFor(edgePosition = edgePosition, viewport = viewport).isNoOp) {
      stop()
      return
    }

    activeRequest = EditorEdgeAutoScrollRequest(edgePosition = edgePosition, onScroll = onScroll)
    if (!running) {
      start(context)
    }
  }

  private fun track(
    edgePosition: Offset,
    dispatchPosition: Offset,
    context: EditorGestureContext,
    dispatch: (EditorEdgeAutoScrollDispatch) -> Boolean,
  ) {
    trackWithResult(
      edgePosition = edgePosition,
      context = context,
      onScroll = { result ->
        dispatchScrolledPosition(
          edgePosition = edgePosition,
          result = result,
          dispatchPosition = dispatchPosition,
          dispatch = dispatch,
          context = context,
        )
      },
    )
  }

  private fun start(context: EditorGestureContext) {
    running = true
    context.effects.launchInteraction {
      try {
        while (true) {
          delay(EditorEdgeAutoScrollTickMillis)
          val request = activeRequest ?: break
          val viewport = context.geometry.resolveEdgeAutoScrollViewport() ?: break
          val plan =
            planFor(
              edgePosition = request.edgePosition + request.accumulatedScroll,
              viewport = viewport,
            )
          if (plan.isNoOp) {
            break
          }

          val delta =
            Offset(
              x =
                plan.horizontalDirection *
                  plan.horizontalSpeedPxPerSec *
                  EditorEdgeAutoScrollTickMillis / 1000f,
              y =
                plan.verticalDirection *
                  plan.verticalSpeedPxPerSec *
                  EditorEdgeAutoScrollTickMillis / 1000f,
            )
          val consumed = context.effects.dispatchEdgeAutoScroll(delta)
          if (consumed == Offset.Zero) {
            continue
          }

          request.onScroll(
            EditorEdgeAutoScrollResult(
              requestedDelta = delta,
              consumedDelta = consumed,
              viewport = context.geometry.resolveEdgeAutoScrollViewport() ?: viewport,
            )
          )
          if (activeRequest === request) {
            activeRequest = request.copy(accumulatedScroll = request.accumulatedScroll + consumed)
          }
        }
      } finally {
        running = false
        activeRequest = null
        lastDispatchedPoint = null
      }
    }
  }

  private fun dispatchScrolledPosition(
    edgePosition: Offset,
    result: EditorEdgeAutoScrollResult,
    dispatchPosition: Offset,
    dispatch: (EditorEdgeAutoScrollDispatch) -> Boolean,
    context: EditorGestureContext,
  ) {
    val viewport = result.viewport
    val rect = viewport.rect
    val verticalDirection =
      sign(result.requestedDelta.y).takeIf { result.consumedDelta.y != 0f } ?: 0f
    val horizontalDirection =
      sign(result.requestedDelta.x).takeIf { result.consumedDelta.x != 0f } ?: 0f
    val verticalBoundaryReached =
      reachedScrollBoundary(
        requestedDelta = result.requestedDelta.y,
        consumedDelta = result.consumedDelta.y,
      )
    val horizontalBoundaryReached =
      reachedScrollBoundary(
        requestedDelta = result.requestedDelta.x,
        consumedDelta = result.consumedDelta.x,
      )
    var x = dispatchPosition.x
    var y = dispatchPosition.y
    if (verticalDirection > 0f) {
      y = if (verticalBoundaryReached) rect.bottom else rect.bottom - viewport.edgeThresholdPx
    } else if (verticalDirection < 0f) {
      y = if (verticalBoundaryReached) rect.top else rect.top + viewport.edgeThresholdPx
    }
    if (horizontalDirection > 0f) {
      x = if (horizontalBoundaryReached) rect.right else rect.right - viewport.edgeThresholdPx
    } else if (horizontalDirection < 0f) {
      x = if (horizontalBoundaryReached) rect.left else rect.left + viewport.edgeThresholdPx
    }

    val position = Offset(x = x, y = y)
    val point = context.geometry.resolvePoint(position) ?: return
    if (point.page < 0 || point == lastDispatchedPoint) {
      return
    }
    lastDispatchedPoint = point
    if (
      dispatch(
        EditorEdgeAutoScrollDispatch(
          edgePosition = edgePosition,
          dispatchPosition = position,
          point = point,
        )
      )
    ) {
      context.semantics.magnifier.show(position)
    }
  }

  private fun planFor(edgePosition: Offset, viewport: EditorEdgeAutoScrollViewport) =
    computeEdgeAutoScrollPlan(
      pointer = edgePosition,
      insetViewport = viewport.rect,
      edgeThresholdPx = viewport.edgeThresholdPx,
      minSpeedPxPerSec = viewport.minSpeedPxPerSecond,
      maxSpeedPxPerSec = viewport.maxSpeedPxPerSecond,
    )

  fun stop() {
    activeRequest = null
    lastDispatchedPoint = null
  }

  fun reset() {
    stop()
  }
}

private fun reachedScrollBoundary(requestedDelta: Float, consumedDelta: Float): Boolean =
  requestedDelta != 0f && consumedDelta != 0f && abs(consumedDelta) < abs(requestedDelta) - 0.001f

private data class EditorEdgeAutoScrollRequest(
  val edgePosition: Offset,
  val accumulatedScroll: Offset = Offset.Zero,
  val onScroll: (EditorEdgeAutoScrollResult) -> Unit,
)

private data class EditorEdgeAutoScrollResult(
  val requestedDelta: Offset,
  val consumedDelta: Offset,
  val viewport: EditorEdgeAutoScrollViewport,
)

internal data class EditorEdgeAutoScrollDispatch(
  val edgePosition: Offset,
  val dispatchPosition: Offset,
  val point: PagePoint,
)

private val EditorEdgeAutoScrollViewport.edgeThresholdPx: Float
  get() = EditorEdgeAutoScrollThresholdDp * density

private val EditorEdgeAutoScrollViewport.minSpeedPxPerSecond: Float
  get() = EditorEdgeAutoScrollMinSpeedDpPerSecond * density

private val EditorEdgeAutoScrollViewport.maxSpeedPxPerSecond: Float
  get() = EditorEdgeAutoScrollMaxSpeedDpPerSecond * density
