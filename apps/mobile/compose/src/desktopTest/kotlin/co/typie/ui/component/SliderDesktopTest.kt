package co.typie.ui.component

import androidx.compose.foundation.gestures.awaitEachGesture
import androidx.compose.foundation.gestures.awaitFirstDown
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.size
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableFloatStateOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.input.pointer.PointerEventPass
import androidx.compose.ui.input.pointer.changedToUp
import androidx.compose.ui.input.pointer.pointerInput
import androidx.compose.ui.input.pointer.positionChange
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.test.ExperimentalTestApi
import androidx.compose.ui.test.onNodeWithTag
import androidx.compose.ui.test.performMouseInput
import androidx.compose.ui.test.performTouchInput
import androidx.compose.ui.test.runComposeUiTest
import androidx.compose.ui.unit.dp
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertFalse
import kotlin.test.assertTrue

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

  @Test
  fun activeDragCommitsWhenAncestorConsumesReleaseOutsideTrack() = runComposeUiTest {
    var committedValue: Float? = null
    var canceled = false

    setContent {
      Box(Modifier.size(width = 280.dp, height = 32.dp).consumeReleaseAtInitialPass()) {
        Slider(
          value = 10f,
          range = 0f..100f,
          onDragStart = {},
          onDrag = {},
          onDragEnd = { committedValue = it },
          onDragCancel = { canceled = true },
          thumbSize = 0.dp,
          modifier = Modifier.testTag(ConsumedReleaseSliderTag).size(width = 200.dp, height = 32.dp),
        )
      }
    }
    waitForIdle()

    onNodeWithTag(ConsumedReleaseSliderTag).performTouchInput {
      val y = center.y
      down(Offset(x = width * 0.1f, y = y))
      moveTo(Offset(x = width * 0.7f, y = y))
      moveTo(Offset(x = width + 48f, y = y))
      up()
    }
    waitForIdle()

    assertFalse(canceled)
    assertEquals(100f, committedValue)
  }

  @Test
  fun consumedReleaseWithoutDragCancelsInsteadOfCommittingTap() = runComposeUiTest {
    var committedValue: Float? = null
    var canceled = false

    setContent {
      Box(Modifier.size(width = 200.dp, height = 32.dp).consumeReleaseAtInitialPass()) {
        Slider(
          value = 10f,
          range = 0f..100f,
          onDragStart = {},
          onDrag = {},
          onDragEnd = { committedValue = it },
          onDragCancel = { canceled = true },
          thumbSize = 0.dp,
          modifier = Modifier.testTag(ConsumedTapReleaseSliderTag).fillMaxSize(),
        )
      }
    }
    waitForIdle()

    onNodeWithTag(ConsumedTapReleaseSliderTag).performTouchInput {
      down(Offset(x = width * 0.1f, y = center.y))
      up()
    }
    waitForIdle()

    assertEquals(null, committedValue)
    assertTrue(canceled)
  }

  @Test
  fun dragStartProvidesLatestDisplayedValueWhenDraftDiffersFromNodeValue() = runComposeUiTest {
    var nodeValue by mutableFloatStateOf(85f)
    var draftValue by mutableStateOf<Float?>(null)

    setContent {
      val currentValue = draftValue ?: nodeValue
      Slider(
        value = currentValue,
        range = 0f..100f,
        onDragStart = { initialValue -> draftValue = initialValue },
        onDrag = { draftValue = it },
        onDragEnd = {
          nodeValue = it
          draftValue = null
        },
        onDragCancel = { draftValue = null },
        thumbSize = 20.dp,
        modifier = Modifier.testTag(LatestDraftSliderTag).size(width = 200.dp, height = 32.dp),
      )
    }
    waitForIdle()
    runOnIdle { draftValue = 100f }
    waitForIdle()

    val slider = onNodeWithTag(LatestDraftSliderTag)
    slider.performMouseInput {
      moveTo(Offset(x = 190f, y = center.y))
      press()
      moveTo(Offset(x = 248f, y = center.y))
    }
    runOnIdle { assertEquals(100f, draftValue) }

    slider.performMouseInput { release() }
    runOnIdle {
      assertEquals(100f, nodeValue)
      assertEquals(null, draftValue)
    }
  }

  private companion object {
    const val SliderTag = "slider-track"
    const val ConsumedSliderTag = "consumed-slider-track"
    const val ConsumedReleaseSliderTag = "consumed-release-slider-track"
    const val ConsumedTapReleaseSliderTag = "consumed-tap-release-slider-track"
    const val LatestDraftSliderTag = "latest-draft-slider-track"
  }
}

private fun Modifier.consumeReleaseAtInitialPass(): Modifier =
  pointerInput(Unit) {
    awaitEachGesture {
      awaitFirstDown(requireUnconsumed = false, pass = PointerEventPass.Initial)

      while (true) {
        val event = awaitPointerEvent(PointerEventPass.Initial)
        val change = event.changes.first()

        if (!change.pressed) {
          change.consume()
          break
        }
      }
    }
  }
