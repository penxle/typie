package co.typie.navigation

import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.input.nestedscroll.NestedScrollConnection
import androidx.compose.ui.input.nestedscroll.NestedScrollSource
import androidx.compose.ui.unit.Velocity

internal class NavigationPopNestedScroll : NestedScrollConnection {
  private val gestureSession = NavigationPopGestureSession()
  private var activationDistance = 0f
  private var canStart: () -> Boolean = { false }
  private var onStart: () -> Unit = {}
  private var onDrag: (Float) -> Unit = {}
  private var onRelease: (Float) -> Unit = {}
  private var onCancel: () -> Unit = {}
  private var scrollInterruption: ScrollInterruption? = null
  private var pressedPointerId: Long? = null
  private var pointerDownPosition: Offset? = null
  private var pointerPosition: Offset? = null

  val isCurrentSequenceRejected: Boolean
    get() = gestureSession.isCurrentSequenceRejected

  fun update(
    activationDistance: Float,
    canStart: () -> Boolean,
    onStart: () -> Unit,
    onDrag: (Float) -> Unit,
    onRelease: (Float) -> Unit,
    onCancel: () -> Unit,
  ) {
    this.activationDistance = activationDistance
    this.canStart = canStart
    this.onStart = onStart
    this.onDrag = onDrag
    this.onRelease = onRelease
    this.onCancel = onCancel
  }

  fun registerScrollInterruption(
    owner: Any,
    isScrollInProgress: () -> Boolean,
    interrupt: () -> Unit,
  ) {
    scrollInterruption = ScrollInterruption(owner, isScrollInProgress, interrupt)
  }

  fun unregisterScrollInterruption(owner: Any) {
    if (scrollInterruption?.owner === owner) {
      scrollInterruption = null
    }
  }

  fun updatePressedDragPointerCount(
    count: Int,
    downInSystemBackZone: Boolean = false,
    pointerId: Long? = null,
    position: Offset? = null,
  ) {
    val isFirstDown = count == 1 && !gestureSession.hasPressedDragPointer
    when {
      isFirstDown -> {
        pressedPointerId = pointerId
        pointerDownPosition = position
        pointerPosition = position
      }
      count == 1 && pointerId == pressedPointerId -> pointerPosition = position
      count == 0 -> {
        pressedPointerId = null
        pointerDownPosition = null
        pointerPosition = null
      }
    }
    val currentScrollInterruption = scrollInterruption
    val interruptsScrolling =
      isFirstDown && currentScrollInterruption?.isScrollInProgress?.invoke() == true
    val wasClaimed = gestureSession.isClaimed
    gestureSession.updatePressedDragPointerCount(count, downInSystemBackZone)
    if (interruptsScrolling) {
      gestureSession.rejectCurrentSequence()
      currentScrollInterruption.interrupt()
    }
    if (wasClaimed && !gestureSession.isClaimed) {
      onCancel()
    }
  }

  fun finishDirectGesture(velocityX: Float) {
    gestureSession.reset()
    onRelease(velocityX)
  }

  fun cancelDirectGesture() {
    gestureSession.reset()
    onCancel()
  }

  fun cancel() {
    if (!gestureSession.isClaimed) {
      return
    }
    gestureSession.reset()
    onCancel()
  }

  override fun onPreScroll(available: Offset, source: NestedScrollSource): Offset {
    if (source != NestedScrollSource.UserInput || !gestureSession.isClaimed) {
      return Offset.Zero
    }

    onDrag(available.x)
    return available
  }

  override fun onPostScroll(
    consumed: Offset,
    available: Offset,
    source: NestedScrollSource,
  ): Offset {
    if (
      source != NestedScrollSource.UserInput ||
        !gestureSession.hasPressedDragPointer ||
        gestureSession.isClaimed ||
        gestureSession.isCurrentSequenceRejected ||
        !canStart()
    ) {
      return Offset.Zero
    }
    if (consumed != Offset.Zero) {
      gestureSession.rejectCurrentSequence()
      return Offset.Zero
    }

    val dragFromStart =
      pointerPosition?.let { current -> pointerDownPosition?.let { down -> current - down } }
        ?: return Offset.Zero
    val activation = resolveNavigationPopActivation(dragFromStart, activationDistance)
    when (activation) {
      NavigationPopActivation.Pending -> return Offset.Zero
      NavigationPopActivation.Rejected -> {
        gestureSession.rejectCurrentSequence()
        return Offset.Zero
      }
      is NavigationPopActivation.Ready -> {
        if (!gestureSession.tryClaim(initialDrag = dragFromStart, childConsumed = false)) {
          return Offset.Zero
        }

        onStart()
        onDrag(activation.overshootX)
      }
    }
    return available
  }

  override suspend fun onPreFling(available: Velocity): Velocity =
    if (releaseClaimedGesture(available.x)) available else Velocity.Zero

  override suspend fun onPostFling(consumed: Velocity, available: Velocity): Velocity =
    if (releaseClaimedGesture(consumed.x + available.x)) available else Velocity.Zero

  private fun releaseClaimedGesture(velocityX: Float): Boolean {
    if (!gestureSession.isClaimed) {
      return false
    }
    gestureSession.reset()
    onRelease(velocityX)
    return true
  }

  private class ScrollInterruption(
    val owner: Any,
    val isScrollInProgress: () -> Boolean,
    val interrupt: () -> Unit,
  )
}
