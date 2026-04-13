package co.typie.ext

import androidx.compose.runtime.Composable
import androidx.compose.runtime.DisposableEffect
import androidx.compose.runtime.remember
import androidx.compose.ui.awt.ComposeWindow
import java.awt.event.MouseAdapter
import java.awt.event.MouseEvent
import java.awt.event.MouseMotionAdapter
import java.awt.event.MouseWheelEvent
import javax.swing.SwingUtilities
import kotlin.math.abs

private const val TOUCH_SLOP_PX = 5
private const val DRAG_SCROLL_SENSITIVITY = 0.25
private const val SYSTEM_CHROME_TOP_PX = 60

@Composable
fun DesktopScrollTranslation(window: ComposeWindow, content: @Composable () -> Unit) {
  val awtScale = remember(window) { window.graphicsConfiguration?.defaultTransform?.scaleX ?: 1.0 }

  DisposableEffect(window) {
    val handler = DragToScrollHandler(window, awtScale)
    handler.install()
    onDispose { handler.uninstall() }
  }

  content()
}

private class DragToScrollHandler(private val window: ComposeWindow, private val awtScale: Double) {
  private var startX = 0
  private var startY = 0
  private var lastY = 0
  private var scrolling = false
  private var inSystemChrome = false

  private val mouseListener =
    object : MouseAdapter() {
      override fun mousePressed(e: MouseEvent) {
        startX = e.x
        startY = e.y
        lastY = e.y
        scrolling = false
        inSystemChrome = e.y < SYSTEM_CHROME_TOP_PX
      }

      override fun mouseReleased(e: MouseEvent) {
        scrolling = false
      }
    }

  private val motionListener =
    object : MouseMotionAdapter() {
      override fun mouseDragged(e: MouseEvent) {
        if (inSystemChrome) return

        val dy = e.y - lastY
        lastY = e.y

        if (!scrolling) {
          val totalDx = abs(e.x - startX)
          val totalDy = abs(e.y - startY)
          if (totalDx < TOUCH_SLOP_PX && totalDy < TOUCH_SLOP_PX) return
          scrolling = true
        }

        if (dy == 0) return

        val wheelEvent =
          MouseWheelEvent(
            e.component,
            MouseWheelEvent.MOUSE_WHEEL,
            e.`when`,
            e.modifiersEx,
            e.x,
            e.y,
            e.xOnScreen,
            e.yOnScreen,
            0,
            false,
            MouseWheelEvent.WHEEL_UNIT_SCROLL,
            1,
            0,
            -dy.toDouble() / awtScale * DRAG_SCROLL_SENSITIVITY,
          )
        val target = e.component
        SwingUtilities.invokeLater { target.dispatchEvent(wheelEvent) }
      }
    }

  fun install() {
    window.addMouseListener(mouseListener)
    window.addMouseMotionListener(motionListener)
  }

  fun uninstall() {
    window.removeMouseListener(mouseListener)
    window.removeMouseMotionListener(motionListener)
  }
}
