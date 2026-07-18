package co.typie.editor.interaction

import androidx.compose.animation.core.VectorConverter
import androidx.compose.animation.core.animate
import androidx.compose.foundation.MutatePriority
import androidx.compose.foundation.gestures.FlingBehavior
import androidx.compose.foundation.gestures.Scroll2DScope
import androidx.compose.foundation.gestures.ScrollScope
import androidx.compose.foundation.gestures.Scrollable2DState
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.input.nestedscroll.NestedScrollDispatcher
import androidx.compose.ui.input.nestedscroll.NestedScrollSource
import androidx.compose.ui.unit.Velocity
import co.typie.editor.interaction.gestures.EditorPanGestureDriver
import co.typie.editor.viewport.editorViewportWheelScrollDeltaPx
import kotlin.math.pow
import kotlin.math.sqrt
import kotlinx.coroutines.CoroutineStart
import kotlinx.coroutines.Job
import kotlinx.coroutines.channels.Channel
import kotlinx.coroutines.launch

internal class EditorViewportScrollDriver(
  private val scrollableState: () -> Scrollable2DState?,
  private val nestedScrollDispatcher: () -> NestedScrollDispatcher?,
  private val flingBehavior: () -> FlingBehavior?,
  private val touchSlopProvider: () -> Float,
  private val maximumFlingVelocityProvider: () -> Float,
  private val launch: (suspend () -> Unit) -> Job,
  private val onCancel: () -> Unit = {},
) : EditorPanGestureDriver {
  private var activePan: ActivePan? = null
  private var activeSelfFling: ActiveSelfFling? = null

  val isAvailable: Boolean
    get() = scrollableState() != null && nestedScrollDispatcher() != null

  override val shouldCatchTouch: Boolean
    get() = activeSelfFling != null || scrollableState()?.isScrollInProgress == true

  override val touchSlop: Float
    get() = touchSlopProvider()

  override val maximumFlingVelocity: Float
    get() = maximumFlingVelocityProvider()

  override fun start(): Boolean {
    val state = scrollableState() ?: return false
    val dispatcher = nestedScrollDispatcher() ?: return false
    val currentFlingBehavior = flingBehavior()
    val precedingSelfFling = activeSelfFling
    cancel()

    val pan =
      ActivePan(
        state = state,
        dispatcher = dispatcher,
        flingBehavior = currentFlingBehavior,
        deltas = Channel(Channel.UNLIMITED),
        precedingSelfFling = precedingSelfFling,
      )
    activePan = pan
    pan.job = launch {
      try {
        state.scroll(MutatePriority.UserInput) {
          pan.precedingSelfFling?.job?.join()
          for (delta in pan.deltas) {
            dispatchScroll(
              delta = delta,
              dispatcher = dispatcher,
              source = NestedScrollSource.UserInput,
            )
          }
        }
      } finally {
        pan.deltas.cancel()
        if (activePan === pan) {
          activePan = null
          if (pan.panStarted) {
            onCancel()
          }
        }
      }
    }
    return true
  }

  override fun markPanStarted() {
    activePan?.panStarted = true
  }

  override fun update(delta: Offset) {
    activePan?.deltas?.trySend(delta)
  }

  override fun end(velocity: Velocity) {
    val pan = activePan ?: return
    activePan = null
    pan.deltas.close()
    val selfFling = ActiveSelfFling()
    activeSelfFling = selfFling
    selfFling.job =
      pan.dispatcher.coroutineScope.launch(start = CoroutineStart.UNDISPATCHED) {
        try {
          pan.job.join()
          performFling(
            state = pan.state,
            dispatcher = pan.dispatcher,
            flingBehavior = pan.flingBehavior,
            initialVelocity = velocity,
          )
        } finally {
          if (activeSelfFling === selfFling) {
            activeSelfFling = null
          }
        }
      }
  }

  override fun cancel() {
    val pan = activePan ?: return
    activePan = null
    pan.deltas.cancel()
    pan.job.cancel()
    if (pan.panStarted) {
      onCancel()
    }
  }

  fun launchSemanticsScroll(offset: Offset) {
    launch { performSemanticsScroll(offset) }
  }

  fun launchPointerSignalScroll(scrollDelta: Offset, density: Float): Boolean {
    val delta = editorViewportWheelScrollDeltaPx(scrollDelta = scrollDelta, density = density)
    return launchIndirectScroll(delta)
  }

  fun launchTrackpadPan(panOffset: Offset): Boolean = launchIndirectScroll(panOffset)

  suspend fun performSemanticsScroll(offset: Offset): Offset {
    val state = scrollableState() ?: return Offset.Zero
    val dispatcher = nestedScrollDispatcher() ?: return Offset.Zero
    var previousValue = Offset.Zero
    state.scroll(MutatePriority.Default) {
      animate(Offset.VectorConverter, Offset.Zero, offset) { currentValue, _ ->
        val delta = currentValue - previousValue
        val consumed =
          dispatchScroll(
            delta = delta,
            dispatcher = dispatcher,
            source = NestedScrollSource.SideEffect,
          )
        previousValue += consumed
      }
    }
    return previousValue
  }

  private fun launchIndirectScroll(delta: Offset): Boolean {
    val state = scrollableState() ?: return false
    val dispatcher = nestedScrollDispatcher() ?: return false
    if (delta == Offset.Zero || !delta.x.isFinite() || !delta.y.isFinite()) {
      return false
    }
    launch {
      state.scroll(MutatePriority.UserInput) {
        // Indirect input has deltas but no editor-owned drag/fling terminal. Do not let a nested
        // parent claim it as a user drag that can never be released.
        dispatchScroll(
          delta = delta,
          dispatcher = dispatcher,
          source = NestedScrollSource.SideEffect,
        )
      }
    }
    return true
  }

  private suspend fun performFling(
    state: Scrollable2DState,
    dispatcher: NestedScrollDispatcher,
    flingBehavior: FlingBehavior?,
    initialVelocity: Velocity,
  ) {
    val preConsumed = dispatcher.dispatchPreFling(initialVelocity)
    val available = initialVelocity - preConsumed
    var velocityLeft = available
    val selfFlingJob =
      if (flingBehavior == null || available == Velocity.Zero) {
        null
      } else {
        launch { velocityLeft = performFlingAnimation(state, dispatcher, flingBehavior, available) }
      }
    selfFlingJob?.join()
    dispatcher.dispatchPostFling(consumed = available - velocityLeft, available = velocityLeft)
  }
}

private class ActivePan(
  val state: Scrollable2DState,
  val dispatcher: NestedScrollDispatcher,
  val flingBehavior: FlingBehavior?,
  val deltas: Channel<Offset>,
  val precedingSelfFling: ActiveSelfFling?,
) {
  lateinit var job: Job
  var panStarted = false
}

private class ActiveSelfFling {
  lateinit var job: Job
}

private fun Scroll2DScope.dispatchScroll(
  delta: Offset,
  dispatcher: NestedScrollDispatcher,
  source: NestedScrollSource,
): Offset {
  val preConsumed = dispatcher.dispatchPreScroll(delta, source)
  val available = delta - preConsumed
  val selfConsumed = scrollBy(available)
  val remaining = available - selfConsumed
  val postConsumed = dispatcher.dispatchPostScroll(selfConsumed, remaining, source)
  return preConsumed + selfConsumed + postConsumed
}

private suspend fun performFlingAnimation(
  state: Scrollable2DState,
  dispatcher: NestedScrollDispatcher,
  flingBehavior: FlingBehavior,
  velocity: Velocity,
): Velocity {
  if (velocity == Velocity.Zero) {
    return Velocity.Zero
  }
  var velocityLeft = velocity
  state.scroll(MutatePriority.Default) {
    val scroll2DScope = this
    val flingScope =
      object : ScrollScope {
        override fun scrollBy(pixels: Float): Float {
          val delta = pixels.toOffset(velocity)
          val consumed =
            scroll2DScope.dispatchScroll(
              delta = delta,
              dispatcher = dispatcher,
              source = NestedScrollSource.SideEffect,
            )
          return consumed.magnitude
        }
      }
    val remainingMagnitude =
      with(flingBehavior) { with(flingScope) { performFling(velocity.magnitude) } }
    velocityLeft = remainingMagnitude.toVelocity(velocity)
  }
  return velocityLeft
}

private val Offset.magnitude: Float
  get() = sqrt(x.pow(2) + y.pow(2))

private val Velocity.magnitude: Float
  get() = sqrt(x.pow(2) + y.pow(2))

private fun Float.toOffset(direction: Velocity): Offset {
  val magnitude = direction.magnitude
  if (magnitude <= 0f || !magnitude.isFinite()) {
    return Offset.Zero
  }
  return Offset(x = this * direction.x / magnitude, y = this * direction.y / magnitude)
}

private fun Float.toVelocity(direction: Velocity): Velocity {
  val magnitude = direction.magnitude
  return if (magnitude <= 0f || !magnitude.isFinite()) {
    Velocity(0f, this)
  } else {
    Velocity(x = this * direction.x / magnitude, y = this * direction.y / magnitude)
  }
}
