package co.typie.editor.interaction.gestures

import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.input.pointer.PointerInputChange
import androidx.compose.ui.input.pointer.util.VelocityTracker
import androidx.compose.ui.input.pointer.util.addPointerInputChange
import androidx.compose.ui.unit.Velocity
import co.typie.editor.interaction.EditorGestureContext
import co.typie.editor.interaction.EditorInteractionEvent
import co.typie.editor.interaction.canApply

internal interface EditorPanGestureDriver {
  val shouldCatchTouch: Boolean
  val touchSlop: Float
  val maximumFlingVelocity: Float

  fun start(): Boolean

  fun markPanStarted()

  fun update(delta: Offset)

  fun end(velocity: Velocity)

  fun cancel()
}

internal class EditorPanGesture {
  private val velocityTracker = VelocityTracker()
  private var session: Session? = null

  private var active = false

  val hasPendingPointer: Boolean
    get() = session != null && !active

  fun prepareFresh(change: PointerInputChange, position: Offset, driver: EditorPanGestureDriver) {
    if (session != null) {
      return
    }
    prepare(change = change, position = position, startKind = StartKind.Fresh, driver = driver)
  }

  fun prepareScrollCatch(
    change: PointerInputChange,
    position: Offset,
    driver: EditorPanGestureDriver,
  ): Boolean {
    if (session != null || !driver.shouldCatchTouch) {
      return false
    }
    prepare(
      change = change,
      position = position,
      startKind = StartKind.ScrollCatch,
      driver = driver,
    )
    if (!driver.start()) {
      clear()
      return false
    }
    session?.driverStarted = true
    return true
  }

  fun resume(change: PointerInputChange, position: Offset, driver: EditorPanGestureDriver) {
    reset()
    prepare(change = change, position = position, startKind = StartKind.Resumed, driver = driver)
  }

  fun update(change: PointerInputChange, position: Offset, context: EditorGestureContext): Boolean {
    val current = session?.takeIf { it.pointerId == change.id.value } ?: return false
    val pressed = change.pressed
    val delta = position - current.lastPosition

    if (active) {
      if (delta != Offset.Zero) {
        current.driver.update(delta)
      }
      trackVelocity(position = position, change = change)
      current.lastPosition = position
      if (!pressed) {
        finish(context = context)
      }
      return true
    }

    if (!pressed || change.isConsumed) {
      val caughtScroll = current.startKind == StartKind.ScrollCatch
      if (current.driverStarted) {
        current.driver.cancel()
      }
      clear()
      return caughtScroll
    }

    if (current.startKind != StartKind.Fresh) {
      if (delta == Offset.Zero) {
        return current.startKind == StartKind.ScrollCatch
      }
      if (!start(context = context)) {
        if (current.driverStarted) {
          current.driver.cancel()
        }
        clear()
        return false
      }
      trackVelocity(position = position, change = change)
      current.driver.update(delta)
      current.lastPosition = position
      return true
    }

    trackVelocity(position = position, change = change)
    val fromStart = position - current.startPosition
    val distance = fromStart.getDistance()
    val touchSlop = current.driver.touchSlop.coerceAtLeast(0f)
    if (distance <= touchSlop) {
      current.lastPosition = position
      return false
    }
    if (!start(context = context)) {
      clear()
      return false
    }
    val overSlop =
      if (distance > 0f) {
        fromStart * ((distance - touchSlop) / distance)
      } else {
        Offset.Zero
      }
    if (overSlop != Offset.Zero) {
      current.driver.update(overSlop)
    }
    current.lastPosition = position
    return true
  }

  fun cancel(context: EditorGestureContext) {
    val current = session ?: return
    if (current.driverStarted) {
      current.driver.cancel()
    }
    if (active) {
      context.applyModeEvent(EditorInteractionEvent.PanCancel)
    }
    clear()
  }

  fun reset() {
    session?.takeIf { current -> current.driverStarted }?.driver?.cancel()
    clear()
  }

  private fun prepare(
    change: PointerInputChange,
    position: Offset,
    startKind: StartKind,
    driver: EditorPanGestureDriver,
  ) {
    session =
      Session(
        pointerId = change.id.value,
        startPosition = position,
        lastPosition = position,
        startKind = startKind,
        driver = driver,
      )
    velocityTracker.resetTracking()
    trackVelocity(position = position, change = change)
  }

  private fun start(context: EditorGestureContext): Boolean {
    val current = session ?: return false
    if (!context.mode.canApply(EditorInteractionEvent.PanStart)) {
      return false
    }
    if (!current.driverStarted) {
      if (!current.driver.start()) {
        return false
      }
      current.driverStarted = true
    }
    current.driver.markPanStarted()
    active = true
    context.applyModeEvent(EditorInteractionEvent.PanStart)
    return true
  }

  private fun finish(context: EditorGestureContext) {
    val current = session ?: return
    val maximum =
      current.driver.maximumFlingVelocity.takeIf { it.isFinite() && it > 0f } ?: Float.MAX_VALUE
    val velocity = velocityTracker.calculateVelocity(Velocity(maximum, maximum))
    current.driver.end(velocity)
    context.applyModeEvent(EditorInteractionEvent.PanEnd)
    clear()
  }

  private fun clear() {
    session = null
    active = false
    velocityTracker.resetTracking()
  }

  private fun trackVelocity(position: Offset, change: PointerInputChange) {
    velocityTracker.addPointerInputChange(event = change, offset = position - change.position)
  }

  private data class Session(
    val pointerId: Long,
    val startPosition: Offset,
    var lastPosition: Offset,
    val startKind: StartKind,
    val driver: EditorPanGestureDriver,
    var driverStarted: Boolean = false,
  )

  private enum class StartKind {
    Fresh,
    Resumed,
    ScrollCatch,
  }
}
