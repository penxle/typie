package co.typie.ext

import androidx.compose.runtime.Composable
import androidx.compose.runtime.CompositionLocalProvider
import androidx.compose.runtime.DisposableEffect
import androidx.compose.runtime.remember
import androidx.compose.ui.awt.ComposeWindow
import java.awt.Component
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

private enum class Axis {
  VERTICAL,
  HORIZONTAL,
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
  private var axis: Axis? = null
  private var flingDelta = 0.0
  private var flingTimer: Timer? = null
  private var lastWheelTarget: Component? = null
  private var lastWheelX = 0
  private var lastWheelY = 0
  private var lastWheelXOnScreen = 0
  private var lastWheelYOnScreen = 0
  private var lastWheelModifiers = 0

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
        axis = null
        flingDelta = 0.0
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
          axis = if (totalDx >= totalDy) Axis.HORIZONTAL else Axis.VERTICAL
          scrolling = true
        }

        val (delta, modifiers) =
          when (axis) {
            Axis.HORIZONTAL -> {
              if (dx == 0) return
              (-dx.toDouble() / awtScale * DRAG_SCROLL_SENSITIVITY) to
                (e.modifiersEx or InputEvent.SHIFT_DOWN_MASK)
            }
            Axis.VERTICAL -> {
              if (dy == 0) return
              (-dy.toDouble() / awtScale * DRAG_SCROLL_SENSITIVITY) to e.modifiersEx
            }
            null -> return
          }

        trackFlingDelta(e, delta, modifiers)
        val wheelEvent =
          MouseWheelEvent(
            e.component,
            MouseWheelEvent.MOUSE_WHEEL,
            e.`when`,
            modifiers,
            e.x,
            e.y,
            e.xOnScreen,
            e.yOnScreen,
            0,
            false,
            MouseWheelEvent.WHEEL_UNIT_SCROLL,
            1,
            0,
            delta,
          )
        val target = e.component
        SwingUtilities.invokeLater { target.dispatchEvent(wheelEvent) }
      }
    }

  private fun trackFlingDelta(e: MouseEvent, delta: Double, modifiers: Int) {
    flingDelta =
      if (flingDelta == 0.0) {
        delta
      } else {
        flingDelta * (1.0 - FLING_SMOOTHING) + delta * FLING_SMOOTHING
      }
    flingDelta = flingDelta.coerceIn(-FLING_MAX_DELTA, FLING_MAX_DELTA)
    lastWheelTarget = e.component
    lastWheelX = e.x
    lastWheelY = e.y
    lastWheelXOnScreen = e.xOnScreen
    lastWheelYOnScreen = e.yOnScreen
    val buttonMask =
      InputEvent.BUTTON1_DOWN_MASK or InputEvent.BUTTON2_DOWN_MASK or InputEvent.BUTTON3_DOWN_MASK
    lastWheelModifiers = modifiers and buttonMask.inv()
  }

  private fun startFling() {
    val target = lastWheelTarget ?: return
    var delta = flingDelta * FLING_DELTA_MULTIPLIER
    if (abs(delta) < FLING_MIN_DELTA) {
      return
    }

    stopFling()
    val timer =
      Timer(FLING_FRAME_MS) {
        delta *= FLING_DECAY
        if (abs(delta) < FLING_MIN_DELTA) {
          stopFling()
          return@Timer
        }

        target.dispatchEvent(
          MouseWheelEvent(
            target,
            MouseWheelEvent.MOUSE_WHEEL,
            System.currentTimeMillis(),
            lastWheelModifiers,
            lastWheelX,
            lastWheelY,
            lastWheelXOnScreen,
            lastWheelYOnScreen,
            0,
            false,
            MouseWheelEvent.WHEEL_UNIT_SCROLL,
            1,
            0,
            delta,
          )
        )
      }
    flingTimer = timer
    timer.start()
  }

  private fun stopFling() {
    flingTimer?.stop()
    flingTimer = null
  }

  fun install() {
    window.addMouseListener(mouseListener)
    window.addMouseMotionListener(motionListener)
  }

  fun uninstall() {
    stopFling()
    window.removeMouseListener(mouseListener)
    window.removeMouseMotionListener(motionListener)
  }
}
