package co.typie.ui.component

import androidx.compose.foundation.gestures.awaitEachGesture
import androidx.compose.foundation.gestures.awaitFirstDown
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.size
import androidx.compose.ui.Modifier
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.input.pointer.PointerEventPass
import androidx.compose.ui.input.pointer.changedToUp
import androidx.compose.ui.input.pointer.pointerInput
import androidx.compose.ui.input.pointer.positionChange
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.test.ExperimentalTestApi
import androidx.compose.ui.test.onNodeWithTag
import androidx.compose.ui.test.performTouchInput
import androidx.compose.ui.test.runComposeUiTest
import androidx.compose.ui.unit.dp
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertFalse

@OptIn(ExperimentalTestApi::class)
class SliderDesktopTest {
  @Test
  fun activeDragCommitsPointerPositionFromReleaseEvent() = runComposeUiTest {
    var committedValue: Float? = null

    setContent {
      Slider(
        value = 10f,
        range = 0f..100f,
        onDragStart = {},
        onDrag = {},
        onDragEnd = { committedValue = it },
        thumbSize = 0.dp,
        modifier = Modifier.testTag(SliderTag).size(width = 200.dp, height = 32.dp),
      )
    }
    waitForIdle()

    onNodeWithTag(SliderTag).performTouchInput {
      val y = center.y
      down(Offset(x = width * 0.1f, y = y))
      moveTo(Offset(x = width * 0.4f, y = y))
      updatePointerTo(pointerId = 0, position = Offset(x = width.toFloat(), y = y))
      up()
    }
    waitForIdle()

    assertEquals(100f, committedValue)
  }

  @Test
  fun activeDragContinuesWhenAncestorConsumesLaterMove() = runComposeUiTest {
    var committedValue: Float? = null
    var canceled = false

    setContent {
      Box(
        Modifier.size(width = 200.dp, height = 32.dp).pointerInput(Unit) {
          awaitEachGesture {
            awaitFirstDown(requireUnconsumed = false, pass = PointerEventPass.Initial)
            var moveEvents = 0

            while (true) {
              val event = awaitPointerEvent(PointerEventPass.Initial)
              val change = event.changes.first()

              if (change.changedToUp()) {
                break
              }
              if (change.positionChange() != Offset.Zero) {
                moveEvents += 1
                if (moveEvents >= 2) {
                  change.consume()
                }
              }
            }
          }
        }
      ) {
        Slider(
          value = 10f,
          range = 0f..100f,
          onDragStart = {},
          onDrag = {},
          onDragEnd = { committedValue = it },
          onDragCancel = { canceled = true },
          thumbSize = 0.dp,
          modifier = Modifier.testTag(ConsumedSliderTag).fillMaxSize(),
        )
      }
    }
    waitForIdle()

    onNodeWithTag(ConsumedSliderTag).performTouchInput {
      val y = center.y
      down(Offset(x = width * 0.1f, y = y))
      moveTo(Offset(x = width * 0.4f, y = y))
      moveTo(Offset(x = width * 0.7f, y = y))
      up()
    }
    waitForIdle()

    assertFalse(canceled)
    assertEquals(70f, committedValue)
  }

  private companion object {
    const val SliderTag = "slider-track"
    const val ConsumedSliderTag = "consumed-slider-track"
  }
}
