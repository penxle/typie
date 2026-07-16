package co.typie.navigation

import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.input.nestedscroll.NestedScrollConnection
import androidx.compose.ui.input.nestedscroll.NestedScrollSource
import androidx.compose.ui.unit.Velocity

internal class NavigationPopNestedScroll : NestedScrollConnection {
  private val gestureSession = NavigationPopGestureSession()
  private var canStart: () -> Boolean = { false }
  private var onStart: () -> Unit = {}
  private var onDrag: (Float) -> Unit = {}
  private var onRelease: () -> Unit = {}
  private var onCancel: () -> Unit = {}

  val isMultiTouchRejected: Boolean
    get() = gestureSession.isMultiTouchRejected

  val isSystemBackZoneRejected: Boolean
    get() = gestureSession.isSystemBackZoneRejected

  fun update(
    canStart: () -> Boolean,
    onStart: () -> Unit,
    onDrag: (Float) -> Unit,
    onRelease: () -> Unit,
    onCancel: () -> Unit,
  ) {
    this.canStart = canStart
    this.onStart = onStart
    this.onDrag = onDrag
    this.onRelease = onRelease
    this.onCancel = onCancel
  }

  fun updatePressedDragPointerCount(count: Int, downInSystemBackZone: Boolean = false) {
    if (gestureSession.updatePressedDragPointerCount(count, downInSystemBackZone)) {
      onCancel()
    }
  }

  fun finishDirectGesture() {
    gestureSession.reset()
    onRelease()
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
        !canStart()
    ) {
      return Offset.Zero
    }
    if (
      !gestureSession.tryClaim(
        initialDrag = consumed + available,
        childConsumed = consumed != Offset.Zero,
      )
    ) {
      return Offset.Zero
    }

    onStart()
    onDrag(available.x)
    return available
  }

  override suspend fun onPreFling(available: Velocity): Velocity =
    if (releaseClaimedGesture()) available else Velocity.Zero

  override suspend fun onPostFling(consumed: Velocity, available: Velocity): Velocity =
    if (releaseClaimedGesture()) available else Velocity.Zero

  private fun releaseClaimedGesture(): Boolean {
    if (!gestureSession.isClaimed) {
      return false
    }
    gestureSession.reset()
    onRelease()
    return true
  }
}
