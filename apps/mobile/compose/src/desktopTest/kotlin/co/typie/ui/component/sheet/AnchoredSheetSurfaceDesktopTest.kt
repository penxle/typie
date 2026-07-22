package co.typie.ui.component.sheet

import androidx.compose.foundation.clickable
import androidx.compose.foundation.gestures.Orientation
import androidx.compose.foundation.gestures.rememberScrollableState
import androidx.compose.foundation.gestures.scrollable
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.size
import androidx.compose.ui.Modifier
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.input.pointer.PointerEventPass
import androidx.compose.ui.input.pointer.PointerEventType
import androidx.compose.ui.input.pointer.pointerInput
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.test.ExperimentalTestApi
import androidx.compose.ui.test.onNodeWithTag
import androidx.compose.ui.test.performClick
import androidx.compose.ui.test.performMouseInput
import androidx.compose.ui.test.performTouchInput
import androidx.compose.ui.test.runComposeUiTest
import androidx.compose.ui.test.swipe
import androidx.compose.ui.unit.dp
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertTrue

@OptIn(ExperimentalTestApi::class)
class AnchoredSheetSurfaceDesktopTest {
  @Test
  fun intrinsicSheetWithoutScrimAllowsViewportTouchScrollAboveSheet() = runComposeUiTest {
    var viewportScroll = 0f

    setContent {
      Box(Modifier.size(width = 400.dp, height = 800.dp)) {
        val scrollableState = rememberScrollableState { delta ->
          viewportScroll += -delta
          delta
        }
        Box(
          Modifier.testTag(ViewportTag)
            .fillMaxSize()
            .scrollable(state = scrollableState, orientation = Orientation.Vertical)
        )
        AnchoredSheetSurface(
          stops = emptyList(),
          stopPolicy = SheetStop.Policy.KeepAll,
          onDismissed = {},
        ) {
          Box(Modifier.fillMaxWidth().height(160.dp))
        }
      }
    }
    waitForIdle()

    onNodeWithTag(ViewportTag).performTouchInput {
      swipe(start = Offset(x = width / 2f, y = 240f), end = Offset(x = width / 2f, y = 80f))
    }
    waitForIdle()

    assertTrue(viewportScroll > 0f)
  }

  @Test
  fun intrinsicSheetWithoutScrimAllowsViewportWheelScrollAboveSheet() = runComposeUiTest {
    var viewportScroll = 0f

    setContent {
      Box(Modifier.size(width = 400.dp, height = 800.dp)) {
        Box(
          Modifier.testTag(ViewportTag).fillMaxSize().pointerInput(Unit) {
            awaitPointerEventScope {
              while (true) {
                val event = awaitPointerEvent(PointerEventPass.Main)
                if (event.type == PointerEventType.Scroll) {
                  viewportScroll += event.changes.sumOf { it.scrollDelta.y.toDouble() }.toFloat()
                  event.changes.forEach { it.consume() }
                }
              }
            }
          }
        )
        AnchoredSheetSurface(
          stops = emptyList(),
          stopPolicy = SheetStop.Policy.KeepAll,
          onDismissed = {},
        ) {
          Box(Modifier.fillMaxWidth().height(160.dp))
        }
      }
    }
    waitForIdle()

    onNodeWithTag(ViewportTag).performMouseInput {
      moveTo(Offset(x = width / 2f, y = 160f))
      scroll(Offset(x = 0f, y = 120f))
    }
    waitForIdle()

    assertTrue(viewportScroll > 0f)
  }

  @Test
  fun dismissReportsStartBeforeDismissed() = runComposeUiTest {
    val events = mutableListOf<String>()

    setContent {
      Box(Modifier.size(width = 400.dp, height = 800.dp)) {
        AnchoredSheetSurface(
          stops = emptyList(),
          stopPolicy = SheetStop.Policy.KeepAll,
          onDismissStarted = { events += "start" },
          onDismissed = { events += "dismissed" },
        ) {
          Box(Modifier.testTag(DismissTag).fillMaxWidth().height(160.dp).clickable { dismiss() })
        }
      }
    }
    waitForIdle()

    onNodeWithTag(DismissTag).performClick()

    waitUntil { "dismissed" in events }
    assertEquals(listOf("start", "dismissed"), events)
  }

  @Test
  fun reversingDismissDragReportsCancellation() = runComposeUiTest {
    val events = mutableListOf<String>()

    setContent {
      Box(Modifier.size(width = 400.dp, height = 800.dp)) {
        AnchoredSheetSurface(
          stops = emptyList(),
          stopPolicy = SheetStop.Policy.KeepAll,
          onDismissStarted = { events += "start" },
          onDismissCancelled = { events += "cancelled" },
          onDismissed = { events += "dismissed" },
        ) {
          Box(Modifier.testTag(SheetTag).fillMaxWidth().height(160.dp))
        }
      }
    }
    waitForIdle()

    onNodeWithTag(SheetTag).performTouchInput {
      down(center)
      moveBy(Offset(0f, 140f), delayMillis = 500)
    }
    waitUntil { "start" in events }

    onNodeWithTag(SheetTag).performTouchInput { moveBy(Offset(0f, -140f), delayMillis = 500) }
    waitUntil { "cancelled" in events }

    onNodeWithTag(SheetTag).performTouchInput { up() }
    waitForIdle()

    assertEquals(listOf("start", "cancelled"), events)
  }

  private companion object {
    const val ViewportTag = "viewport"
    const val DismissTag = "dismiss"
    const val SheetTag = "sheet"
  }
}
