package co.typie.editor.interaction.semantics

import androidx.compose.ui.geometry.Offset
import co.typie.editor.PagePoint
import co.typie.editor.ffi.Position
import co.typie.editor.interaction.EditorEdgeAutoScrollViewport
import co.typie.editor.interaction.EditorGestureContext
import co.typie.ext.computeEdgeAutoScrollPlan
import kotlin.math.abs
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
    context: EditorGestureContext,
  ) {
    track(
      edgePosition = edgePosition,
      dispatchPosition = dispatchPosition,
      context = context,
      dispatch = { point -> context.editor.dispatchSelectionHandleExtension(point, anchor) },
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
      dispatch = { point ->
        val selectionContext = context.semantics.selectionExpansion.context(context.editor)
        selectionContext != null &&
          context.editor.dispatchSelectionExtension(point = point, context = selectionContext)
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
      dispatch = { point ->
        context.semantics.cursorMove.enqueuePrimaryClick(
          editor = context.editor,
          point = point,
          clickCount = 1,
        )
      },
    )
  }

  private fun track(
    edgePosition: Offset,
    dispatchPosition: Offset,
    context: EditorGestureContext,
    dispatch: (PagePoint) -> Boolean,
  ) {
    val viewport = context.geometry.resolveEdgeAutoScrollViewport()
    if (viewport == null || planFor(edgePosition = edgePosition, viewport = viewport).isNoOp) {
      stop()
      return
    }

    activeRequest =
      EditorEdgeAutoScrollRequest(
        edgePosition = edgePosition,
        dispatchPosition = dispatchPosition,
        dispatch = dispatch,
      )
    if (!running) {
      start(context)
    }
  }

  private fun start(context: EditorGestureContext) {
    running = true
    context.effects.launchInteraction {
      try {
        while (true) {
          delay(EditorEdgeAutoScrollTickMillis)
          val request = activeRequest ?: break
          val viewport = context.geometry.resolveEdgeAutoScrollViewport() ?: break
          val plan = planFor(edgePosition = request.edgePosition, viewport = viewport)
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

          val scrolledViewport = context.geometry.resolveEdgeAutoScrollViewport() ?: viewport
          dispatchScrolledPosition(
            request = request,
            viewport = scrolledViewport,
            verticalDirection = plan.verticalDirection.takeIf { consumed.y != 0f } ?: 0f,
            verticalBoundaryReached =
              reachedScrollBoundary(requestedDelta = delta.y, consumedDelta = consumed.y),
            horizontalDirection = plan.horizontalDirection.takeIf { consumed.x != 0f } ?: 0f,
            horizontalBoundaryReached =
              reachedScrollBoundary(requestedDelta = delta.x, consumedDelta = consumed.x),
            context = context,
          )
        }
      } finally {
        running = false
        activeRequest = null
        lastDispatchedPoint = null
      }
    }
  }

  private fun dispatchScrolledPosition(
    request: EditorEdgeAutoScrollRequest,
    viewport: EditorEdgeAutoScrollViewport,
    verticalDirection: Float,
    verticalBoundaryReached: Boolean,
    horizontalDirection: Float,
    horizontalBoundaryReached: Boolean,
    context: EditorGestureContext,
  ) {
    val rect = viewport.rect
    var x = request.dispatchPosition.x
    var y = request.dispatchPosition.y
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

    val point = context.geometry.resolvePoint(Offset(x = x, y = y)) ?: return
    if (point.page < 0 || point == lastDispatchedPoint) {
      return
    }
    lastDispatchedPoint = point
    if (request.dispatch(point)) {
      context.semantics.magnifier.show(Offset(x = x, y = y))
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
  val dispatchPosition: Offset,
  val dispatch: (PagePoint) -> Boolean,
)

private val EditorEdgeAutoScrollViewport.edgeThresholdPx: Float
  get() = EditorEdgeAutoScrollThresholdDp * density

private val EditorEdgeAutoScrollViewport.minSpeedPxPerSecond: Float
  get() = EditorEdgeAutoScrollMinSpeedDpPerSecond * density

private val EditorEdgeAutoScrollViewport.maxSpeedPxPerSecond: Float
  get() = EditorEdgeAutoScrollMaxSpeedDpPerSecond * density
