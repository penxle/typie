package co.typie.editor.external

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
import androidx.compose.ui.platform.LocalViewConfiguration
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.test.ExperimentalTestApi
import androidx.compose.ui.test.onNodeWithTag
import androidx.compose.ui.test.performTouchInput
import androidx.compose.ui.test.runComposeUiTest
import androidx.compose.ui.unit.dp
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertTrue

@OptIn(ExperimentalTestApi::class)
class EditorImageResizeHandleDesktopTest {
  @Test
  fun release_applies_final_up_event_delta_before_commit() = runComposeUiTest {
    val deltas = mutableListOf<Float>()
    var ended = false

    setContent {
      Box(
        Modifier.testTag(HandleTag)
          .size(width = 200.dp, height = 32.dp)
          .imageResizeHandlePointerInput(
            key = false,
            onStart = {},
            onDrag = { deltas += it },
            onEnd = { ended = true },
          )
      )
    }
    waitForIdle()

    onNodeWithTag(HandleTag).performTouchInput {
      val y = center.y
      down(Offset(x = width * 0.1f, y = y))
      moveTo(Offset(x = width * 0.4f, y = y))
      updatePointerTo(pointerId = 0, position = Offset(x = width.toFloat(), y = y))
      up()
    }
    waitForIdle()

    assertEquals(120f, deltas.last())
    assertTrue(ended)
  }

  @Test
  fun cancellation_commits_the_latest_delivered_draft() = runComposeUiTest {
    val deltas = mutableListOf<Float>()
    var draft = 0f
    var committed: Float? = null
    var touchSlop = 0f
    var requestedDelta = 0f

    setContent {
      touchSlop = LocalViewConfiguration.current.touchSlop
      Box(
        Modifier.size(width = 200.dp, height = 32.dp).pointerInput(Unit) {
          awaitEachGesture {
            awaitFirstDown(requireUnconsumed = false, pass = PointerEventPass.Initial)
            var moveEvents = 0
            while (true) {
              val change = awaitPointerEvent(PointerEventPass.Initial).changes.first()
              if (change.changedToUp()) break
              if (change.positionChange() != Offset.Zero) {
                moveEvents += 1
                if (moveEvents >= 2) change.consume()
              }
            }
          }
        }
      ) {
        Box(
          Modifier.testTag(CancelHandleTag)
            .fillMaxSize()
            .imageResizeHandlePointerInput(
              key = false,
              onStart = {},
              onDrag = {
                deltas += it
                draft += it
              },
              onEnd = { committed = draft },
            )
        )
      }
    }
    waitForIdle()

    onNodeWithTag(CancelHandleTag).performTouchInput {
      val y = center.y
      requestedDelta = width * 0.3f
      down(Offset(x = width * 0.1f, y = y))
      moveTo(Offset(x = width * 0.4f, y = y))
      moveTo(Offset(x = width * 0.7f, y = y))
      up()
    }
    waitForIdle()

    assertEquals(1, deltas.size)
    assertEquals(requestedDelta - touchSlop, deltas.single(), absoluteTolerance = 0.001f)
    assertEquals(deltas.sum(), committed)
  }

  private companion object {
    const val HandleTag = "image-resize-handle"
    const val CancelHandleTag = "cancel-image-resize-handle"
  }
}
