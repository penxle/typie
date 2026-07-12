package co.typie.ext

import androidx.compose.ui.awt.ComposeWindow
import java.awt.AWTEvent
import java.awt.Component
import java.awt.event.AWTEventListener
import java.awt.event.InputEvent
import java.awt.event.MouseEvent
import java.awt.event.MouseListener
import java.awt.event.MouseMotionListener
import java.awt.event.MouseWheelEvent
import javax.swing.SwingUtilities
import javax.swing.Timer
import kotlin.test.Test
import kotlin.test.assertEquals

class DesktopScrollTranslationTest {
  @Test
  fun diagonalMouseDragDispatchesBothScrollAxes() {
    val target = WheelEventRecorder()
    lateinit var window: ComposeWindow
    lateinit var handler: Any

    SwingUtilities.invokeAndWait {
      window = ComposeWindow()
      handler = createDragToScrollHandler(window)

      handler
        .field<MouseListener>("mouseListener")
        .mousePressed(mouseEvent(target, MouseEvent.MOUSE_PRESSED, x = 0, y = 100))
      handler
        .field<MouseMotionListener>("motionListener")
        .mouseDragged(mouseEvent(target, MouseEvent.MOUSE_DRAGGED, x = 40, y = 130))
    }
    SwingUtilities.invokeAndWait {}

    try {
      assertDiagonalWheelEvents(target)
    } finally {
      SwingUtilities.invokeAndWait { window.dispose() }
    }
  }

  @Test
  fun mouseReleaseKeepsPendingDragScroll() {
    val target = WheelEventRecorder()
    lateinit var window: ComposeWindow
    lateinit var handler: Any

    SwingUtilities.invokeAndWait {
      window = ComposeWindow()
      handler = createDragToScrollHandler(window)
      val mouseListener = handler.field<MouseListener>("mouseListener")

      mouseListener.mousePressed(mouseEvent(target, MouseEvent.MOUSE_PRESSED, x = 0, y = 100))
      handler
        .field<MouseMotionListener>("motionListener")
        .mouseDragged(mouseEvent(target, MouseEvent.MOUSE_DRAGGED, x = 40, y = 130))
      mouseListener.mouseReleased(mouseEvent(target, MouseEvent.MOUSE_RELEASED, x = 40, y = 130))
      handler.field<Timer?>("flingTimer")?.stop()
    }
    SwingUtilities.invokeAndWait {}

    try {
      assertDiagonalWheelEvents(target)
    } finally {
      SwingUtilities.invokeAndWait { window.dispose() }
    }
  }

  @Test
  fun childPressCancelsPendingDragScroll() {
    val target = WheelEventRecorder()
    lateinit var window: ComposeWindow
    lateinit var handler: Any

    SwingUtilities.invokeAndWait {
      window = ComposeWindow()
      handler = createDragToScrollHandler(window)
      val childPressTarget = object : Component() {}
      window.add(childPressTarget)
      val mouseListener = handler.field<MouseListener>("mouseListener")

      mouseListener.mousePressed(mouseEvent(target, MouseEvent.MOUSE_PRESSED, x = 0, y = 100))
      handler
        .field<MouseMotionListener>("motionListener")
        .mouseDragged(mouseEvent(target, MouseEvent.MOUSE_DRAGGED, x = 40, y = 130))
      handler
        .field<AWTEventListener>("awtEventListener")
        .eventDispatched(mouseEvent(childPressTarget, MouseEvent.MOUSE_PRESSED, x = 0, y = 100))
    }
    SwingUtilities.invokeAndWait {}

    try {
      assertEquals(0, target.wheelEvents.size)
    } finally {
      SwingUtilities.invokeAndWait { window.dispose() }
    }
  }
}

private fun assertDiagonalWheelEvents(target: WheelEventRecorder) {
  assertEquals(
    expected = 2,
    actual = target.wheelEvents.size,
    message = target.wheelEvents.joinToString { "${it.preciseWheelRotation}:${it.modifiersEx}" },
  )
  val horizontal = target.wheelEvents.single { it.modifiersEx and InputEvent.SHIFT_DOWN_MASK != 0 }
  val vertical = target.wheelEvents.single { it.modifiersEx and InputEvent.SHIFT_DOWN_MASK == 0 }
  assertEquals(-10.0, horizontal.preciseWheelRotation)
  assertEquals(-7.5, vertical.preciseWheelRotation)
}

private fun createDragToScrollHandler(window: ComposeWindow): Any {
  val handlerClass = Class.forName("co.typie.ext.DragToScrollHandler")
  return handlerClass.declaredConstructors.single().run {
    isAccessible = true
    newInstance(window, 1.0, { false })
  }
}

private inline fun <reified T> Any.field(name: String): T =
  javaClass.getDeclaredField(name).run {
    isAccessible = true
    get(this@field) as T
  }

private class WheelEventRecorder : Component() {
  val wheelEvents = mutableListOf<MouseWheelEvent>()

  init {
    enableEvents(AWTEvent.MOUSE_WHEEL_EVENT_MASK)
  }

  override fun processMouseWheelEvent(event: MouseWheelEvent) {
    wheelEvents += event
  }
}

private fun mouseEvent(target: Component, id: Int, x: Int, y: Int): MouseEvent =
  MouseEvent(
    target,
    id,
    System.currentTimeMillis(),
    InputEvent.BUTTON1_DOWN_MASK,
    x,
    y,
    1,
    false,
    MouseEvent.BUTTON1,
  )
