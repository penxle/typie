package co.typie.ui.component

import kotlin.test.Test
import kotlin.test.assertEquals

class SliderTest {
  @Test
  fun cancel_discards_started_drag_without_commit_value() {
    val events = mutableListOf<String>()
    val gesture =
      SliderGestureSession(
        initialValue = 30f,
        valueFromX = { it },
        onDragStart = { events += "start" },
        onDrag = { events += "drag:$it" },
      )

    gesture.start()
    gesture.updateAt(45f)
    gesture.cancel()

    assertEquals(null, gesture.release())
    assertEquals(listOf("start", "drag:45.0"), events)
  }

  @Test
  fun release_after_drag_commits_current_value() {
    val events = mutableListOf<String>()
    val gesture =
      SliderGestureSession(
        initialValue = 30f,
        valueFromX = { it },
        onDragStart = { events += "start" },
        onDrag = { events += "drag:$it" },
      )

    gesture.start()
    gesture.updateAt(45f)

    assertEquals(45f, gesture.release())
    assertEquals(listOf("start", "drag:45.0"), events)
  }

  @Test
  fun release_after_fractional_drag_commits_fractional_value() {
    val events = mutableListOf<String>()
    val gesture =
      SliderGestureSession(
        initialValue = 30f,
        valueFromX = { it / 10f },
        onDragStart = { events += "start" },
        onDrag = { events += "drag:$it" },
      )

    gesture.start()
    gesture.updateAt(454f)

    assertEquals(45.4f, gesture.release())
    assertEquals(listOf("start", "drag:45.4"), events)
  }
}
