package co.typie.editor.interaction.gestures

import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.input.pointer.util.VelocityTracker
import androidx.compose.ui.unit.Velocity
import co.typie.editor.interaction.EditorGestureContext
import co.typie.editor.interaction.EditorInteractionEvent
import co.typie.editor.interaction.canApply

private const val PointerMoveStoppedTimeoutMillis = 40L

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

  fun prepareFresh(
    pointerId: Long,
    position: Offset,
    nowMillis: Long,
    driver: EditorPanGestureDriver,
  ) {
    if (session != null) {
      return
    }
    prepare(
      pointerId = pointerId,
      position = position,
      nowMillis = nowMillis,
      startKind = StartKind.Fresh,
      driver = driver,
    )
  }

  fun prepareScrollCatch(
    pointerId: Long,
    position: Offset,
    nowMillis: Long,
    driver: EditorPanGestureDriver,
  ): Boolean {
    if (session != null || !driver.shouldCatchTouch) {
      return false
    }
    prepare(
      pointerId = pointerId,
      position = position,
      nowMillis = nowMillis,
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

  fun resume(pointerId: Long, position: Offset, nowMillis: Long, driver: EditorPanGestureDriver) {
    reset()
    prepare(
      pointerId = pointerId,
      position = position,
      nowMillis = nowMillis,
      startKind = StartKind.Resumed,
      driver = driver,
    )
  }

  fun update(
    pointerId: Long,
    position: Offset,
    nowMillis: Long,
    pressed: Boolean,
    consumed: Boolean,
    context: EditorGestureContext,
  ): Boolean {
    val current = session?.takeIf { it.pointerId == pointerId } ?: return false
    val delta = position - current.lastPosition

    if (active) {
      if (delta != Offset.Zero) {
        current.driver.update(delta)
      }
      if (pressed) {
        velocityTracker.addPosition(nowMillis, position)
        current.lastVelocitySampleAtMillis = nowMillis
      }
      current.lastPosition = position
      if (!pressed) {
        finish(nowMillis = nowMillis, context = context)
      }
      return true
    }

    if (!pressed || consumed) {
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
      velocityTracker.addPosition(nowMillis, position)
      current.lastVelocitySampleAtMillis = nowMillis
      current.driver.update(delta)
      current.lastPosition = position
      return true
    }

    velocityTracker.addPosition(nowMillis, position)
    current.lastVelocitySampleAtMillis = nowMillis
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
    pointerId: Long,
    position: Offset,
    nowMillis: Long,
    startKind: StartKind,
    driver: EditorPanGestureDriver,
  ) {
    session =
      Session(
        pointerId = pointerId,
        startPosition = position,
        lastPosition = position,
        lastVelocitySampleAtMillis = nowMillis,
        startKind = startKind,
        driver = driver,
      )
    velocityTracker.resetTracking()
    velocityTracker.addPosition(nowMillis, position)
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

  private fun finish(nowMillis: Long, context: EditorGestureContext) {
    val current = session ?: return
    val maximum =
      current.driver.maximumFlingVelocity.takeIf { it.isFinite() && it > 0f } ?: Float.MAX_VALUE
    val velocity =
      if (nowMillis - current.lastVelocitySampleAtMillis > PointerMoveStoppedTimeoutMillis) {
        Velocity.Zero
      } else {
        velocityTracker.calculateVelocity(Velocity(maximum, maximum))
      }
    current.driver.end(velocity)
    context.applyModeEvent(EditorInteractionEvent.PanEnd)
    clear()
  }

  private fun clear() {
    session = null
    active = false
    velocityTracker.resetTracking()
  }

  private data class Session(
    val pointerId: Long,
    val startPosition: Offset,
    var lastPosition: Offset,
    var lastVelocitySampleAtMillis: Long,
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
