package co.typie.ui.component

import androidx.compose.ui.hapticfeedback.HapticFeedbackType
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

  @Test
  fun discrete_slider_uses_regular_ticks_through_eight_intervals() {
    assertEquals(HapticFeedbackType.SegmentTick, sliderHapticFeedbackType(0f..8f, 1f, 4f))
  }

  @Test
  fun discrete_slider_uses_frequent_ticks_above_eight_intervals() {
    assertEquals(HapticFeedbackType.SegmentFrequentTick, sliderHapticFeedbackType(0f..9f, 1f, 4f))
  }

  @Test
  fun current_discrete_slider_ranges_use_frequent_ticks() {
    listOf(800f..2400f to 100f, -10f..40f to 5f, 80f..220f to 10f, 0f..100f to 5f).forEach {
      (range, step) ->
      assertEquals(
        HapticFeedbackType.SegmentFrequentTick,
        sliderHapticFeedbackType(range, step, range.start),
      )
    }
  }

  @Test
  fun continuous_slider_uses_gesture_end_only_at_boundaries() {
    assertEquals(null, sliderHapticFeedbackType(0f..100f, null, 50f))
    assertEquals(HapticFeedbackType.GestureEnd, sliderHapticFeedbackType(0f..100f, null, 0f))
    assertEquals(HapticFeedbackType.GestureEnd, sliderHapticFeedbackType(0f..100f, null, 100f))
    assertEquals(null, sliderHapticFeedbackType(0f..100f, 0f, 50f))
    assertEquals(null, sliderHapticFeedbackType(0f..100f, -1f, 50f))
    assertEquals(HapticFeedbackType.GestureEnd, sliderHapticFeedbackType(0f..100f, -1f, 100f))
  }

  @Test
  fun continuous_slider_emits_gesture_end_only_when_entering_a_clamped_boundary() {
    val haptics = mutableListOf<HapticFeedbackType>()
    val gesture =
      SliderGestureSession(
        initialValue = 50f,
        valueFromX = { it.coerceIn(0f, 100f) },
        onDragStart = {},
        onDrag = { value ->
          val haptic: HapticFeedbackType? = sliderHapticFeedbackType(0f..100f, null, value)
          if (haptic != null) {
            haptics += haptic
          }
        },
      )

    gesture.start()
    gesture.updateAt(100f)
    gesture.updateAt(120f)
    gesture.updateAt(80f)
    gesture.updateAt(100f)

    assertEquals(listOf(HapticFeedbackType.GestureEnd, HapticFeedbackType.GestureEnd), haptics)
  }

  @Test
  fun discrete_dense_slider_emits_one_tick_for_a_repeated_endpoint_update() {
    val haptics = mutableListOf<HapticFeedbackType>()
    val gesture =
      SliderGestureSession(
        initialValue = 50f,
        valueFromX = { it.coerceIn(0f, 100f) },
        onDragStart = {},
        onDrag = { value ->
          val haptic: HapticFeedbackType? = sliderHapticFeedbackType(0f..100f, 5f, value)
          if (haptic != null) {
            haptics += haptic
          }
        },
      )

    gesture.start()
    gesture.updateAt(100f)
    gesture.updateAt(120f)

    assertEquals(listOf(HapticFeedbackType.SegmentFrequentTick), haptics)
  }
}
