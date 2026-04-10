@file:JvmName("ModifierDesktopKt")

package co.typie.ext

import androidx.compose.animation.core.animate
import androidx.compose.animation.core.animateDecay
import androidx.compose.animation.core.exponentialDecay
import androidx.compose.animation.core.spring
import androidx.compose.foundation.ScrollState
import androidx.compose.foundation.gestures.Orientation
import androidx.compose.foundation.gestures.ScrollableState
import androidx.compose.foundation.gestures.awaitEachGesture
import androidx.compose.foundation.gestures.awaitFirstDown
import androidx.compose.foundation.gestures.awaitHorizontalTouchSlopOrCancellation
import androidx.compose.foundation.gestures.awaitVerticalTouchSlopOrCancellation
import androidx.compose.foundation.gestures.horizontalDrag
import androidx.compose.foundation.gestures.verticalDrag
import androidx.compose.foundation.horizontalScroll as foundationHorizontalScroll
import androidx.compose.foundation.verticalScroll as foundationVerticalScroll
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableFloatStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.composed
import androidx.compose.ui.draw.clipToBounds
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.graphics.graphicsLayer
import androidx.compose.ui.input.nestedscroll.NestedScrollConnection
import androidx.compose.ui.input.nestedscroll.NestedScrollDispatcher
import androidx.compose.ui.input.nestedscroll.NestedScrollSource
import androidx.compose.ui.input.nestedscroll.nestedScroll
import androidx.compose.ui.input.pointer.AwaitPointerEventScope
import androidx.compose.ui.input.pointer.PointerEventType
import androidx.compose.ui.input.pointer.PointerId
import androidx.compose.ui.input.pointer.PointerInputChange
import androidx.compose.ui.input.pointer.pointerInput
import androidx.compose.ui.input.pointer.util.VelocityTracker
import androidx.compose.ui.unit.Velocity
import kotlin.math.abs
import kotlin.math.min
import kotlin.math.sign
import kotlinx.coroutines.launch

private const val DAMPING = 0.4f
private const val SPRING_STIFFNESS = 400f
private const val FLING_VELOCITY_MULTIPLIER = 0.72f
private const val CONSUMPTION_EPSILON = 0.5f

internal enum class DragScrollFlingMode {
  ScrollableContent,
  DirectBridge,
}

actual fun Modifier.verticalScroll(state: ScrollState, enabled: Boolean): Modifier = composed {
  val isLocked = LocalScrollGestureLockState.current.isLocked
  val dragEnabled = enabled && !isLocked
  val overscrollState = rememberElasticOverscrollState()
  foundationVerticalScroll(state, enabled = dragEnabled)
    .then(
      if (dragEnabled) {
        Modifier.dragScroll(
          state = state,
          axis = ElasticAxis.Vertical,
          overscrollState = overscrollState,
          invertFlingVelocity = true,
        )
      } else {
        Modifier
      }
    )
    .then(
      if (dragEnabled) Modifier.elasticOverscroll(ElasticAxis.Vertical, overscrollState)
      else Modifier
    )
}

actual fun Modifier.horizontalScroll(state: ScrollState, enabled: Boolean): Modifier =
  if (!enabled) {
    composed {
      foundationHorizontalScroll(state, enabled = false).pointerInput(state) {
        awaitPointerEventScope {
          while (true) {
            val event = awaitPointerEvent()
            if (event.type == PointerEventType.Scroll) {
              val change = event.changes.firstOrNull() ?: continue
              val scrollDelta = change.scrollDelta
              val delta = if (scrollDelta.x != 0f) scrollDelta.x else scrollDelta.y
              if (delta != 0f) {
                val consumed = state.dispatchRawDelta(delta)
                if (consumed != 0f) {
                  change.consume()
                }
              }
            }
          }
        }
      }
    }
  } else
    composed {
      val isLocked = LocalScrollGestureLockState.current.isLocked
      if (isLocked) {
        return@composed foundationHorizontalScroll(state, enabled = false)
      }
      val overscrollState = rememberElasticOverscrollState()
      foundationHorizontalScroll(state)
        .then(
          Modifier.dragScroll(
            state = state,
            axis = ElasticAxis.Horizontal,
            overscrollState = overscrollState,
            invertFlingVelocity = true,
          )
        )
        .then(Modifier.elasticOverscroll(ElasticAxis.Horizontal, overscrollState))
    }

internal actual fun Modifier.desktopDragScroll(
  state: ScrollableState,
  orientation: Orientation,
  enabled: Boolean,
): Modifier = composed {
  val isLocked = LocalScrollGestureLockState.current.isLocked
  if (!enabled || isLocked) {
    return@composed this
  }

  this.dragScroll(
    state = state,
    axis = orientation.toElasticAxis(),
    overscrollState = null,
    invertFlingVelocity = false,
  )
}

private enum class ElasticAxis {
  Horizontal,
  Vertical,
}

private class ElasticOverscrollState {
  var offset by mutableFloatStateOf(0f)

  fun consumePre(delta: Float): Float {
    if (offset == 0f || delta == 0f) return 0f

    val wouldReduce = (offset > 0f && delta > 0f) || (offset < 0f && delta < 0f)
    if (!wouldReduce) return 0f

    val maxConsumable = abs(offset) / DAMPING
    val consumed = min(abs(delta), maxConsumable) * sign(delta)
    offset -= consumed * DAMPING
    if (abs(offset) < 0.5f) {
      offset = 0f
    }

    return consumed
  }

  fun applyUnconsumed(delta: Float) {
    if (delta != 0f) {
      offset -= delta * DAMPING
    }
  }

  suspend fun release() {
    if (offset != 0f) {
      animate(offset, 0f, animationSpec = spring(stiffness = SPRING_STIFFNESS)) { value, _ ->
        offset = value
      }
    }
  }
}

@Composable
private fun rememberElasticOverscrollState(): ElasticOverscrollState = remember {
  ElasticOverscrollState()
}

private fun Modifier.dragScroll(
  state: ScrollableState,
  axis: ElasticAxis,
  overscrollState: ElasticOverscrollState?,
  invertFlingVelocity: Boolean = false,
): Modifier = composed {
  val scope = rememberCoroutineScope()
  val decay = remember { exponentialDecay<Float>() }
  val nestedScrollDispatcher = remember { NestedScrollDispatcher() }
  val nestedScrollConnection = remember { object : NestedScrollConnection {} }

  nestedScroll(nestedScrollConnection, nestedScrollDispatcher).pointerInput(
    state,
    axis,
    overscrollState,
    nestedScrollDispatcher,
  ) {
    awaitEachGesture {
      val down = awaitFirstDown(requireUnconsumed = false)
      var ancestorParticipated = false
      var localParticipated = false
      var ancestorConsumedLastSample = false
      var localConsumedLastSample = false
      val velocityTracker =
        VelocityTracker().apply { addPosition(down.uptimeMillis, down.position) }

      val dragStartChange =
        awaitAxisTouchSlopOrCancellation(axis = axis, pointerId = down.id) { change, overSlop ->
          // Match touch-like ownership: initial press may be handled by a child,
          // but actual movement ownership is decided only once drag slop is crossed.
          velocityTracker.addPosition(change.uptimeMillis, change.position)
          change.consume()
          val dispatchResult =
            dispatchDragScrollDelta(
              pointerDelta = overSlop,
              state = state,
              axis = axis,
              nestedScrollDispatcher = nestedScrollDispatcher,
              overscrollState = overscrollState,
            )
          if (abs(dispatchResult.ancestorConsumedPointerDelta) > CONSUMPTION_EPSILON) {
            ancestorParticipated = true
          }
          if (abs(dispatchResult.localConsumedPointerDelta) > CONSUMPTION_EPSILON) {
            localParticipated = true
          }
          ancestorConsumedLastSample =
            abs(dispatchResult.ancestorConsumedPointerDelta) > CONSUMPTION_EPSILON
          localConsumedLastSample =
            abs(dispatchResult.localConsumedPointerDelta) > CONSUMPTION_EPSILON
        }

      if (dragStartChange == null) {
        scope.launch { overscrollState?.release() }
        return@awaitEachGesture
      }

      val dragCompleted =
        dragAlongAxis(axis = axis, pointerId = dragStartChange.id) { change ->
          velocityTracker.addPosition(change.uptimeMillis, change.position)
          val pointerDelta = axis.pointerDelta(change)
          if (pointerDelta == 0f) {
            return@dragAlongAxis
          }

          change.consume()
          val dispatchResult =
            dispatchDragScrollDelta(
              pointerDelta = pointerDelta,
              state = state,
              axis = axis,
              nestedScrollDispatcher = nestedScrollDispatcher,
              overscrollState = overscrollState,
            )
          if (abs(dispatchResult.ancestorConsumedPointerDelta) > CONSUMPTION_EPSILON) {
            ancestorParticipated = true
          }
          if (abs(dispatchResult.localConsumedPointerDelta) > CONSUMPTION_EPSILON) {
            localParticipated = true
          }
          ancestorConsumedLastSample =
            abs(dispatchResult.ancestorConsumedPointerDelta) > CONSUMPTION_EPSILON
          localConsumedLastSample =
            abs(dispatchResult.localConsumedPointerDelta) > CONSUMPTION_EPSILON
        }

      val rawPointerVelocity = axis.extract(velocityTracker.calculateVelocity())
      val flingMode =
        if (invertFlingVelocity) {
          DragScrollFlingMode.ScrollableContent
        } else {
          DragScrollFlingMode.DirectBridge
        }
      val pointerVelocity =
        resolveDragScrollFlingVelocity(pointerVelocity = rawPointerVelocity, mode = flingMode)
      val ancestorHandoffVelocity =
        resolveDragScrollAncestorHandoffVelocity(
          pointerVelocity = rawPointerVelocity,
          mode = flingMode,
        )

      scope.launch {
        finalizeDragScroll(
          pointerVelocity = if (dragCompleted) pointerVelocity else 0f,
          ancestorHandoffVelocity = if (dragCompleted) ancestorHandoffVelocity else 0f,
          ancestorParticipated = ancestorParticipated,
          localParticipated = localParticipated,
          ancestorConsumedLastSample = ancestorConsumedLastSample,
          localConsumedLastSample = localConsumedLastSample,
          axis = axis,
          nestedScrollDispatcher = nestedScrollDispatcher,
          overscrollState = overscrollState,
          decay = decay,
          state = state,
        )
      }
    }
  }
}

internal fun resolveDragScrollFlingVelocity(
  pointerVelocity: Float,
  mode: DragScrollFlingMode,
): Float =
  when (mode) {
    DragScrollFlingMode.ScrollableContent -> -pointerVelocity * FLING_VELOCITY_MULTIPLIER
    DragScrollFlingMode.DirectBridge -> pointerVelocity * FLING_VELOCITY_MULTIPLIER
  }

internal fun resolveDragScrollAncestorHandoffVelocity(
  pointerVelocity: Float,
  mode: DragScrollFlingMode,
): Float =
  when (mode) {
    DragScrollFlingMode.ScrollableContent -> -pointerVelocity
    DragScrollFlingMode.DirectBridge -> pointerVelocity
  }

private data class DragScrollDispatchResult(
  val ancestorConsumedPointerDelta: Float,
  val localConsumedPointerDelta: Float,
  val unconsumedScrollDelta: Float,
)

private fun dispatchDragScrollDelta(
  pointerDelta: Float,
  state: ScrollableState,
  axis: ElasticAxis,
  nestedScrollDispatcher: NestedScrollDispatcher,
  overscrollState: ElasticOverscrollState?,
): DragScrollDispatchResult {
  val scrollDelta = -pointerDelta
  val overscrollPreConsumed = overscrollState?.consumePre(scrollDelta) ?: 0f
  val pointerDeltaAfterOverscroll = -(scrollDelta - overscrollPreConsumed)

  if (pointerDeltaAfterOverscroll == 0f) {
    return DragScrollDispatchResult(
      ancestorConsumedPointerDelta = 0f,
      localConsumedPointerDelta = 0f,
      unconsumedScrollDelta = 0f,
    )
  }

  val preConsumedPointerDelta =
    axis.extract(
      nestedScrollDispatcher.dispatchPreScroll(
        available = axis.offset(pointerDeltaAfterOverscroll),
        source = NestedScrollSource.UserInput,
      )
    )
  val remainingPointerDelta = pointerDeltaAfterOverscroll - preConsumedPointerDelta
  val localConsumedScrollDelta =
    if (remainingPointerDelta != 0f) {
      state.dispatchRawDelta(-remainingPointerDelta)
    } else {
      0f
    }
  val localConsumedPointerDelta = -localConsumedScrollDelta
  val postAvailablePointerDelta = remainingPointerDelta - localConsumedPointerDelta
  val postConsumedPointerDelta =
    axis.extract(
      nestedScrollDispatcher.dispatchPostScroll(
        consumed = axis.offset(localConsumedPointerDelta),
        available = axis.offset(postAvailablePointerDelta),
        source = NestedScrollSource.UserInput,
      )
    )
  val finalAvailablePointerDelta = postAvailablePointerDelta - postConsumedPointerDelta
  val unconsumedScrollDelta = -finalAvailablePointerDelta

  overscrollState?.applyUnconsumed(unconsumedScrollDelta)

  return DragScrollDispatchResult(
    ancestorConsumedPointerDelta = preConsumedPointerDelta + postConsumedPointerDelta,
    localConsumedPointerDelta = localConsumedPointerDelta,
    unconsumedScrollDelta = unconsumedScrollDelta,
  )
}

private fun Orientation.toElasticAxis(): ElasticAxis =
  when (this) {
    Orientation.Horizontal -> ElasticAxis.Horizontal
    Orientation.Vertical -> ElasticAxis.Vertical
  }

private suspend fun AwaitPointerEventScope.awaitAxisTouchSlopOrCancellation(
  axis: ElasticAxis,
  pointerId: PointerId,
  onTouchSlopReached: (PointerInputChange, Float) -> Unit,
): PointerInputChange? =
  when (axis) {
    ElasticAxis.Horizontal -> awaitHorizontalTouchSlopOrCancellation(pointerId, onTouchSlopReached)
    ElasticAxis.Vertical -> awaitVerticalTouchSlopOrCancellation(pointerId, onTouchSlopReached)
  }

private suspend fun AwaitPointerEventScope.dragAlongAxis(
  axis: ElasticAxis,
  pointerId: PointerId,
  onDrag: (PointerInputChange) -> Unit,
): Boolean =
  when (axis) {
    ElasticAxis.Horizontal -> horizontalDrag(pointerId, onDrag)
    ElasticAxis.Vertical -> verticalDrag(pointerId, onDrag)
  }

private fun ElasticAxis.extract(offset: Offset): Float =
  when (this) {
    ElasticAxis.Horizontal -> offset.x
    ElasticAxis.Vertical -> offset.y
  }

private fun ElasticAxis.extract(velocity: Velocity): Float =
  when (this) {
    ElasticAxis.Horizontal -> velocity.x
    ElasticAxis.Vertical -> velocity.y
  }

private fun ElasticAxis.pointerDelta(change: PointerInputChange): Float =
  when (this) {
    ElasticAxis.Horizontal -> change.position.x - change.previousPosition.x
    ElasticAxis.Vertical -> change.position.y - change.previousPosition.y
  }

private fun ElasticAxis.offset(value: Float): Offset =
  when (this) {
    ElasticAxis.Horizontal -> Offset(value, 0f)
    ElasticAxis.Vertical -> Offset(0f, value)
  }

private fun ElasticAxis.velocity(value: Float): Velocity =
  when (this) {
    ElasticAxis.Horizontal -> Velocity(value, 0f)
    ElasticAxis.Vertical -> Velocity(0f, value)
  }

private suspend fun finalizeDragScroll(
  pointerVelocity: Float,
  ancestorHandoffVelocity: Float,
  ancestorParticipated: Boolean,
  localParticipated: Boolean,
  ancestorConsumedLastSample: Boolean,
  localConsumedLastSample: Boolean,
  axis: ElasticAxis,
  nestedScrollDispatcher: NestedScrollDispatcher,
  overscrollState: ElasticOverscrollState?,
  decay: androidx.compose.animation.core.DecayAnimationSpec<Float>,
  state: ScrollableState,
) {
  val shouldHandOffImmediately =
    shouldHandOffDragScrollFlingToAncestorImmediately(
      ancestorParticipated = ancestorParticipated,
      localParticipated = localParticipated,
      ancestorConsumedLastSample = ancestorConsumedLastSample,
      localConsumedLastSample = localConsumedLastSample,
    )

  if (shouldHandOffImmediately) {
    nestedScrollDispatcher.dispatchPostFling(
      consumed = Velocity.Zero,
      available = axis.velocity(ancestorHandoffVelocity),
    )
    overscrollState?.release()
    return
  }

  if (overscrollState != null && abs(overscrollState.offset) > 0.5f) {
    if (ancestorParticipated) {
      nestedScrollDispatcher.dispatchPostFling(consumed = Velocity.Zero, available = Velocity.Zero)
    }
    overscrollState.release()
    return
  }

  if (abs(pointerVelocity) <= 0.5f) {
    if (ancestorParticipated) {
      nestedScrollDispatcher.dispatchPostFling(consumed = Velocity.Zero, available = Velocity.Zero)
    }
    overscrollState?.release()
    return
  }

  var finalAvailableVelocity = axis.velocity(pointerVelocity)
  var previousValue = 0f
  try {
    androidx.compose.animation.core
      .AnimationState(initialValue = 0f, initialVelocity = pointerVelocity)
      .animateDecay(decay) {
        val pointerDelta = value - previousValue
        previousValue = value
        val dispatchResult =
          dispatchDragScrollDelta(
            pointerDelta = pointerDelta,
            state = state,
            axis = axis,
            nestedScrollDispatcher = nestedScrollDispatcher,
            overscrollState = overscrollState,
          )
        if (abs(dispatchResult.unconsumedScrollDelta) > CONSUMPTION_EPSILON) {
          finalAvailableVelocity = axis.velocity(velocity)
          cancelAnimation()
        }
      }
  } finally {
    if (ancestorParticipated) {
      nestedScrollDispatcher.dispatchPostFling(
        consumed = Velocity.Zero,
        available = finalAvailableVelocity,
      )
    }
    overscrollState?.release()
  }
}

internal fun shouldHandOffDragScrollFlingToAncestorImmediately(
  ancestorParticipated: Boolean,
  localParticipated: Boolean,
  ancestorConsumedLastSample: Boolean,
  localConsumedLastSample: Boolean,
): Boolean =
  when {
    ancestorConsumedLastSample && !localConsumedLastSample -> true
    ancestorParticipated && !localParticipated -> true
    else -> false
  }

private fun Modifier.elasticOverscroll(
  axis: ElasticAxis,
  overscrollState: ElasticOverscrollState,
): Modifier = composed {
  val connection =
    remember(axis, overscrollState) {
      object : NestedScrollConnection {
        override fun onPreScroll(available: Offset, source: NestedScrollSource): Offset {
          if (source != NestedScrollSource.UserInput) return Offset.Zero

          val delta = axis.extract(available)
          if (delta == 0f) return Offset.Zero
          return axis.offset(overscrollState.consumePre(delta))
        }

        override fun onPostScroll(
          consumed: Offset,
          available: Offset,
          source: NestedScrollSource,
        ): Offset {
          // Keep elastic overscroll for direct drag gestures handled by dragScroll(),
          // but do not translate the whole container for trackpad or wheel scrolling.
          return Offset.Zero
        }

        override suspend fun onPreFling(available: Velocity): Velocity {
          overscrollState.release()
          return Velocity.Zero
        }

        override suspend fun onPostFling(consumed: Velocity, available: Velocity): Velocity {
          overscrollState.release()
          return Velocity.Zero
        }
      }
    }

  clipToBounds().nestedScroll(connection).graphicsLayer {
    when (axis) {
      ElasticAxis.Horizontal -> translationX = overscrollState.offset
      ElasticAxis.Vertical -> translationY = overscrollState.offset
    }
  }
}
