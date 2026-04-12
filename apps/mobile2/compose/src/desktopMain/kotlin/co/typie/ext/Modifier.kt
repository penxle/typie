@file:JvmName("ModifierDesktopKt")

package co.typie.ext

import androidx.compose.animation.core.animateDecay
import androidx.compose.animation.core.exponentialDecay
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
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.ui.Modifier
import androidx.compose.ui.composed
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
import kotlinx.coroutines.launch

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
      if (dragEnabled) {
        Modifier.elasticOverscroll(ElasticAxis.Vertical, overscrollState)
      } else {
        Modifier
      }
    )
}

actual fun Modifier.horizontalScroll(state: ScrollState, enabled: Boolean): Modifier =
  if (!enabled) {
    composed {
      foundationHorizontalScroll(state, enabled = false).pointerInput(state) {
        awaitPointerEventScope {
          while (true) {
            val event = awaitPointerEvent()
            if (event.type != PointerEventType.Scroll) continue

            val change = event.changes.firstOrNull() ?: continue
            val scrollDelta = change.scrollDelta
            val delta = if (scrollDelta.x != 0f) scrollDelta.x else scrollDelta.y
            if (delta == 0f) continue

            val consumed = state.dispatchRawDelta(delta)
            if (consumed != 0f) {
              change.consume()
            }
          }
        }
      }
    }
  } else {
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
  }

internal actual fun Modifier.desktopDragScroll(
  state: ScrollableState,
  orientation: Orientation,
  enabled: Boolean,
  elasticOverscroll: Boolean,
): Modifier = composed {
  val isLocked = LocalScrollGestureLockState.current.isLocked
  if (!enabled || isLocked) {
    return@composed this
  }

  val axis = orientation.toElasticAxis()
  val overscrollState = rememberElasticOverscrollState()
  val activeOverscrollState = overscrollState.takeIf {
    shouldUseElasticOverscrollForDesktopDragScroll(
      enabled = enabled,
      isLocked = isLocked,
      elasticOverscroll = elasticOverscroll,
    )
  }

  dragScroll(state = state, axis = axis, overscrollState = activeOverscrollState)
    .then(
      if (activeOverscrollState != null) {
        Modifier.elasticOverscroll(axis, activeOverscrollState)
      } else {
        Modifier
      }
    )
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
          ancestorConsumedLastSample =
            abs(dispatchResult.ancestorConsumedPointerDelta) > CONSUMPTION_EPSILON
          localConsumedLastSample =
            abs(dispatchResult.localConsumedPointerDelta) > CONSUMPTION_EPSILON
          ancestorParticipated = ancestorParticipated || ancestorConsumedLastSample
          localParticipated = localParticipated || localConsumedLastSample
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
          ancestorConsumedLastSample =
            abs(dispatchResult.ancestorConsumedPointerDelta) > CONSUMPTION_EPSILON
          localConsumedLastSample =
            abs(dispatchResult.localConsumedPointerDelta) > CONSUMPTION_EPSILON
          ancestorParticipated = ancestorParticipated || ancestorConsumedLastSample
          localParticipated = localParticipated || localConsumedLastSample
        }

      val flingMode =
        if (invertFlingVelocity) {
          DragScrollFlingMode.ScrollableContent
        } else {
          DragScrollFlingMode.DirectBridge
        }
      val ancestorHandoffVelocity =
        resolveDragScrollAncestorHandoffVelocity(
          pointerVelocity = axis.extract(velocityTracker.calculateVelocity()),
          mode = flingMode,
        )

      scope.launch {
        finalizeDragScroll(
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
  allowAncestorDispatch: Boolean = true,
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
    if (allowAncestorDispatch) {
      axis.extract(
        nestedScrollDispatcher.dispatchPreScroll(
          available = axis.offset(pointerDeltaAfterOverscroll),
          source = NestedScrollSource.UserInput,
        )
      )
    } else {
      0f
    }
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
    if (allowAncestorDispatch) {
      axis.extract(
        nestedScrollDispatcher.dispatchPostScroll(
          consumed = axis.offset(localConsumedPointerDelta),
          available = axis.offset(postAvailablePointerDelta),
          source = NestedScrollSource.UserInput,
        )
      )
    } else {
      0f
    }
  val finalAvailablePointerDelta = postAvailablePointerDelta - postConsumedPointerDelta
  val unconsumedScrollDelta = -finalAvailablePointerDelta

  overscrollState?.applyUnconsumed(unconsumedScrollDelta)

  return DragScrollDispatchResult(
    ancestorConsumedPointerDelta = preConsumedPointerDelta + postConsumedPointerDelta,
    localConsumedPointerDelta = localConsumedPointerDelta,
    unconsumedScrollDelta = unconsumedScrollDelta,
  )
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

private suspend fun finalizeDragScroll(
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
  val allowAncestorFlingHandoff =
    shouldAllowDragScrollFlingAncestorHandoff(
      ancestorParticipated = ancestorParticipated,
      ancestorConsumedLastSample = ancestorConsumedLastSample,
      localConsumedLastSample = localConsumedLastSample,
    )
  val preConsumedAncestorVelocity =
    if (allowAncestorFlingHandoff) {
      axis.extract(nestedScrollDispatcher.dispatchPreFling(axis.velocity(ancestorHandoffVelocity)))
    } else {
      0f
    }
  val remainingAncestorVelocity =
    if (allowAncestorFlingHandoff) {
      ancestorHandoffVelocity - preConsumedAncestorVelocity
    } else {
      ancestorHandoffVelocity
    }

  if (allowAncestorFlingHandoff) {
    finalizeAncestorOwnedDragScrollFling(
      remainingAncestorVelocity = remainingAncestorVelocity,
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
  } else {
    finalizeLocalOwnedDragScrollFling(
      decayVelocity = remainingAncestorVelocity * FLING_VELOCITY_MULTIPLIER,
      axis = axis,
      nestedScrollDispatcher = nestedScrollDispatcher,
      overscrollState = overscrollState,
      decay = decay,
      state = state,
    )
  }
}

private suspend fun finalizeAncestorOwnedDragScrollFling(
  remainingAncestorVelocity: Float,
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
  if (abs(remainingAncestorVelocity) <= CONSUMPTION_EPSILON) {
    overscrollState?.release()
    return
  }

  if (
    shouldHandOffDragScrollFlingToAncestorImmediately(
      ancestorParticipated = ancestorParticipated,
      localParticipated = localParticipated,
      ancestorConsumedLastSample = ancestorConsumedLastSample,
      localConsumedLastSample = localConsumedLastSample,
    )
  ) {
    nestedScrollDispatcher.dispatchPostFling(
      consumed = Velocity.Zero,
      available = axis.velocity(remainingAncestorVelocity),
    )
    overscrollState?.release()
    return
  }

  if (overscrollState != null && abs(overscrollState.offset) > CONSUMPTION_EPSILON) {
    if (ancestorParticipated) {
      nestedScrollDispatcher.dispatchPostFling(consumed = Velocity.Zero, available = Velocity.Zero)
    }
    overscrollState.release()
    return
  }

  val decayVelocity = remainingAncestorVelocity * FLING_VELOCITY_MULTIPLIER
  if (abs(decayVelocity) <= CONSUMPTION_EPSILON) {
    if (ancestorParticipated) {
      nestedScrollDispatcher.dispatchPostFling(consumed = Velocity.Zero, available = Velocity.Zero)
    }
    overscrollState?.release()
    return
  }

  var finalAvailableVelocity = Velocity.Zero
  var ancestorConsumedDuringFling = false
  var previousValue = 0f

  try {
    androidx.compose.animation.core
      .AnimationState(initialValue = 0f, initialVelocity = decayVelocity)
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
            allowAncestorDispatch = true,
          )
        if (abs(dispatchResult.ancestorConsumedPointerDelta) > CONSUMPTION_EPSILON) {
          ancestorConsumedDuringFling = true
        }

        if (
          shouldCancelDragScrollDecayForAncestorHandoff(
            ancestorConsumedPointerDelta = dispatchResult.ancestorConsumedPointerDelta,
            localConsumedPointerDelta = dispatchResult.localConsumedPointerDelta,
            unconsumedScrollDelta = dispatchResult.unconsumedScrollDelta,
          ) || abs(dispatchResult.unconsumedScrollDelta) > CONSUMPTION_EPSILON
        ) {
          finalAvailableVelocity = axis.velocity(velocity)
          cancelAnimation()
          return@animateDecay
        }
      }
  } finally {
    if (
      shouldDispatchDragScrollPostFlingToAncestor(
        ancestorParticipated = ancestorParticipated,
        ancestorConsumedDuringFling = ancestorConsumedDuringFling,
        availableVelocity = axis.extract(finalAvailableVelocity),
      )
    ) {
      nestedScrollDispatcher.dispatchPostFling(
        consumed = Velocity.Zero,
        available = finalAvailableVelocity,
      )
    }
    overscrollState?.release()
  }
}

private suspend fun finalizeLocalOwnedDragScrollFling(
  decayVelocity: Float,
  axis: ElasticAxis,
  nestedScrollDispatcher: NestedScrollDispatcher,
  overscrollState: ElasticOverscrollState?,
  decay: androidx.compose.animation.core.DecayAnimationSpec<Float>,
  state: ScrollableState,
) {
  if (overscrollState != null && abs(overscrollState.offset) > CONSUMPTION_EPSILON) {
    overscrollState.release()
    return
  }

  if (abs(decayVelocity) <= CONSUMPTION_EPSILON) {
    overscrollState?.release()
    return
  }

  var finalAvailableVelocity = Velocity.Zero
  var shouldHandOffToAncestor = false
  var previousValue = 0f

  try {
    androidx.compose.animation.core
      .AnimationState(initialValue = 0f, initialVelocity = decayVelocity)
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
            allowAncestorDispatch = false,
          )
        if (abs(dispatchResult.unconsumedScrollDelta) <= CONSUMPTION_EPSILON) {
          return@animateDecay
        }

        when (
          resolveDragScrollBoundaryFlingOutcome(
            availableVelocity = velocity,
            boundaryUnconsumedScrollDelta = dispatchResult.unconsumedScrollDelta,
            overscrollEnabled = overscrollState != null,
          )
        ) {
          DragScrollBoundaryFlingOutcome.HandOffToAncestor -> {
            shouldHandOffToAncestor = true
            finalAvailableVelocity = axis.velocity(velocity)
            overscrollState?.offset = 0f
            cancelAnimation()
            return@animateDecay
          }
          DragScrollBoundaryFlingOutcome.Stop -> {
            cancelAnimation()
            return@animateDecay
          }
          DragScrollBoundaryFlingOutcome.ContinueElasticOverscroll -> {
            overscrollState?.applyUnconsumed(
              resolveDragScrollBoundaryElasticOverscrollDelta(
                availableVelocity = velocity,
                boundaryUnconsumedScrollDelta = dispatchResult.unconsumedScrollDelta,
              )
            )
            cancelAnimation()
            return@animateDecay
          }
        }
      }
  } finally {
    if (shouldHandOffToAncestor) {
      nestedScrollDispatcher.dispatchPostFling(
        consumed = Velocity.Zero,
        available = finalAvailableVelocity,
      )
    }
    overscrollState?.release()
  }
}
