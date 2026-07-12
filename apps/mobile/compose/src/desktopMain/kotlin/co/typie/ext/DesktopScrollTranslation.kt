package co.typie.ext

import androidx.compose.runtime.Composable
import androidx.compose.runtime.CompositionLocalProvider
import androidx.compose.runtime.DisposableEffect
import androidx.compose.runtime.remember
import androidx.compose.ui.awt.ComposeWindow
import java.awt.AWTEvent
import java.awt.Component
import java.awt.Toolkit
import java.awt.event.AWTEventListener
import java.awt.event.InputEvent
import java.awt.event.MouseAdapter
import java.awt.event.MouseEvent
import java.awt.event.MouseMotionAdapter
import java.awt.event.MouseWheelEvent
import javax.swing.SwingUtilities
import javax.swing.Timer
import kotlin.math.abs

private const val TOUCH_SLOP_PX = 5
private const val DRAG_SCROLL_SENSITIVITY = 0.25
private const val SYSTEM_CHROME_TOP_PX = 60
private const val FLING_FRAME_MS = 16
private const val FLING_DECAY = 0.92
private const val FLING_MIN_DELTA = 0.05
private const val FLING_MAX_DELTA = 24.0
private const val FLING_DELTA_MULTIPLIER = 4.0
private const val FLING_SMOOTHING = 0.35

@Composable
fun DesktopScrollTranslation(window: ComposeWindow, content: @Composable () -> Unit) {
  val awtScale = remember(window) { window.graphicsConfiguration?.defaultTransform?.scaleX ?: 1.0 }
  val scrollGestureLockState = remember { ScrollGestureLockState() }

  DisposableEffect(window, scrollGestureLockState) {
    val handler =
      DragToScrollHandler(
        window = window,
        awtScale = awtScale,
        isScrollGestureLocked = { scrollGestureLockState.isLocked },
      )
    handler.install()
    onDispose { handler.uninstall() }
  }

  CompositionLocalProvider(LocalScrollGestureLockState provides scrollGestureLockState) {
    content()
  }
}

private class DragToScrollHandler(
  private val window: ComposeWindow,
  private val awtScale: Double,
  private val isScrollGestureLocked: () -> Boolean,
) {
  private var startX = 0
  private var startY = 0
  private var lastX = 0
  private var lastY = 0
  private var scrolling = false
  private var inSystemChrome = false
  private var flingDeltaX = 0.0
  private var flingDeltaY = 0.0
  private var flingTimer: Timer? = null
  private var lastWheelTarget: Component? = null
  private var lastWheelX = 0
  private var lastWheelY = 0
  private var lastWheelXOnScreen = 0
  private var lastWheelYOnScreen = 0
  private var lastWheelModifiers = 0
  private var flingGeneration = 0L

  private val awtEventListener = AWTEventListener { event ->
    if (event is MouseEvent && event.id == MouseEvent.MOUSE_PRESSED && event.isInWindow()) {
      stopFling()
    }
  }

  private val mouseListener =
    object : MouseAdapter() {
      override fun mousePressed(e: MouseEvent) {
        stopFling()
        startX = e.x
        startY = e.y
        lastX = e.x
        lastY = e.y
        scrolling = false
        inSystemChrome = e.y < SYSTEM_CHROME_TOP_PX
        flingDeltaX = 0.0
        flingDeltaY = 0.0
      }

      override fun mouseReleased(e: MouseEvent) {
        if (scrolling && !inSystemChrome && !isScrollGestureLocked()) {
          startFling()
        }
        scrolling = false
      }
    }

  private val motionListener =
    object : MouseMotionAdapter() {
      override fun mouseDragged(e: MouseEvent) {
        if (inSystemChrome || isScrollGestureLocked()) return

        stopFling()
        val dx = e.x - lastX
        val dy = e.y - lastY
        lastX = e.x
        lastY = e.y

        if (!scrolling) {
          val totalDx = abs(e.x - startX)
          val totalDy = abs(e.y - startY)
          if (totalDx < TOUCH_SLOP_PX && totalDy < TOUCH_SLOP_PX) return
          scrolling = true
        }

        val deltaX = -dx.toDouble() / awtScale * DRAG_SCROLL_SENSITIVITY
        val deltaY = -dy.toDouble() / awtScale * DRAG_SCROLL_SENSITIVITY
        if (deltaX == 0.0 && deltaY == 0.0) return

        trackFlingDelta(e, deltaX, deltaY)
        val target = e.component
        val generation = flingGeneration
        val eventTime = e.`when`
        val modifiers = e.modifiersEx
        val x = e.x
        val y = e.y
        val xOnScreen = e.xOnScreen
        val yOnScreen = e.yOnScreen
        SwingUtilities.invokeLater {
          if (generation == flingGeneration) {
            target.dispatchWheelEvents(
              eventTime = eventTime,
              modifiers = modifiers,
              x = x,
              y = y,
              xOnScreen = xOnScreen,
              yOnScreen = yOnScreen,
              deltaX = deltaX,
              deltaY = deltaY,
            )
          }
        }
      }
    }

  private fun trackFlingDelta(e: MouseEvent, deltaX: Double, deltaY: Double) {
    flingDeltaX = smoothFlingDelta(flingDeltaX, deltaX)
    flingDeltaY = smoothFlingDelta(flingDeltaY, deltaY)
    lastWheelTarget = e.component
    lastWheelX = e.x
    lastWheelY = e.y
    lastWheelXOnScreen = e.xOnScreen
    lastWheelYOnScreen = e.yOnScreen
    val buttonMask =
      InputEvent.BUTTON1_DOWN_MASK or InputEvent.BUTTON2_DOWN_MASK or InputEvent.BUTTON3_DOWN_MASK
    lastWheelModifiers = e.modifiersEx and buttonMask.inv()
  }

  private fun startFling() {
    val target = lastWheelTarget ?: return
    var deltaX = flingDeltaX * FLING_DELTA_MULTIPLIER
    var deltaY = flingDeltaY * FLING_DELTA_MULTIPLIER
    if (abs(deltaX) < FLING_MIN_DELTA && abs(deltaY) < FLING_MIN_DELTA) {
      return
    }

    // The pending drag dispatch and its fling belong to the same input generation.
    val generation = flingGeneration
    val timer =
      Timer(FLING_FRAME_MS) { event ->
        if (generation != flingGeneration) {
          (event.source as? Timer)?.stop()
          return@Timer
        }
        deltaX *= FLING_DECAY
        deltaY *= FLING_DECAY
        val activeDeltaX = deltaX.takeIf { abs(it) >= FLING_MIN_DELTA } ?: 0.0
        val activeDeltaY = deltaY.takeIf { abs(it) >= FLING_MIN_DELTA } ?: 0.0
        if (activeDeltaX == 0.0 && activeDeltaY == 0.0) {
          stopFling()
          return@Timer
        }

        target.dispatchWheelEvents(
          eventTime = System.currentTimeMillis(),
          modifiers = lastWheelModifiers,
          x = lastWheelX,
          y = lastWheelY,
          xOnScreen = lastWheelXOnScreen,
          yOnScreen = lastWheelYOnScreen,
          deltaX = activeDeltaX,
          deltaY = activeDeltaY,
        )
      }
    flingTimer = timer
    timer.start()
  }

  private fun stopFling() {
    flingGeneration++
    flingTimer?.stop()
    flingTimer = null
  }

  private fun smoothFlingDelta(previous: Double, current: Double): Double {
    val smoothed =
      if (previous == 0.0) {
        current
      } else {
        previous * (1.0 - FLING_SMOOTHING) + current * FLING_SMOOTHING
      }
    return smoothed.coerceIn(-FLING_MAX_DELTA, FLING_MAX_DELTA)
  }

  private fun Component.dispatchWheelEvents(
    eventTime: Long,
    modifiers: Int,
    x: Int,
    y: Int,
    xOnScreen: Int,
    yOnScreen: Int,
    deltaX: Double,
    deltaY: Double,
  ) {
    if (deltaY != 0.0) {
      dispatchWheelEvent(
        eventTime = eventTime,
        modifiers = modifiers and InputEvent.SHIFT_DOWN_MASK.inv(),
        x = x,
        y = y,
        xOnScreen = xOnScreen,
        yOnScreen = yOnScreen,
        delta = deltaY,
      )
    }
    if (deltaX != 0.0) {
      dispatchWheelEvent(
        eventTime = eventTime,
        modifiers = modifiers or InputEvent.SHIFT_DOWN_MASK,
        x = x,
        y = y,
        xOnScreen = xOnScreen,
        yOnScreen = yOnScreen,
        delta = deltaX,
      )
    }
  }

  private fun Component.dispatchWheelEvent(
    eventTime: Long,
    modifiers: Int,
    x: Int,
    y: Int,
    xOnScreen: Int,
    yOnScreen: Int,
    delta: Double,
  ) {
    dispatchEvent(
      MouseWheelEvent(
        this,
        MouseWheelEvent.MOUSE_WHEEL,
        eventTime,
        modifiers,
        x,
        y,
        xOnScreen,
        yOnScreen,
        0,
        false,
        MouseWheelEvent.WHEEL_UNIT_SCROLL,
        1,
        0,
        delta,
      )
    )
  }

  private fun MouseEvent.isInWindow(): Boolean =
    component == window || SwingUtilities.getWindowAncestor(component) == window

  fun install() {
    Toolkit.getDefaultToolkit().addAWTEventListener(awtEventListener, AWTEvent.MOUSE_EVENT_MASK)
    window.addMouseListener(mouseListener)
    window.addMouseMotionListener(motionListener)
  }

  fun uninstall() {
    stopFling()
    window.removeMouseListener(mouseListener)
    window.removeMouseMotionListener(motionListener)
    Toolkit.getDefaultToolkit().removeAWTEventListener(awtEventListener)
  }
}
