package co.typie.screen.editor.editor.layout

import androidx.compose.runtime.BroadcastFrameClock
import androidx.compose.ui.MotionDurationScale
import androidx.compose.ui.geometry.Size
import co.typie.editor.scroll.EditorBringIntoViewBehavior
import co.typie.editor.scroll.EditorBringIntoViewPolicy
import co.typie.editor.scroll.EditorBringIntoViewRequests
import co.typie.editor.scroll.EditorBringIntoViewTarget
import co.typie.editor.viewport.EditorViewportState
import kotlin.coroutines.coroutineContext
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertFalse
import kotlin.test.assertTrue
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.ExperimentalCoroutinesApi
import kotlinx.coroutines.test.StandardTestDispatcher
import kotlinx.coroutines.test.TestScope
import kotlinx.coroutines.test.runCurrent
import kotlinx.coroutines.test.runTest

@OptIn(ExperimentalCoroutinesApi::class)
class EditorSmoothScrollSessionTest {
  @Test
  fun `no-op near the active target finishes at the exact target`() =
    runTest(StandardTestDispatcher()) {
      val frameClock = BroadcastFrameClock()
      val viewportState = viewportState()
      val requests = EditorBringIntoViewRequests()
      val session =
        EditorSmoothScrollSession(
          coroutineScope = CoroutineScope(coroutineContext + frameClock),
          viewportState = viewportState,
          bringIntoViewRequests = requests,
        )
      val request = smoothRequest(requests)

      session.retarget(
        request = request,
        targetY = 580f,
        viewportHeight = 400f,
        maximumScrollY = 1600f,
      ) {
        true
      }
      viewportState.scrollToY(targetY = 579.25f, isAutoScroll = true)

      assertTrue(session.finishIfNearTarget(request))
      assertEquals(580f, viewportState.scrollOffset.y)
      assertFalse(session.active)
    }

  @Test
  fun `retains a finished motion when its presentation is stale and retargets it after reconciliation`() =
    runTest(StandardTestDispatcher()) {
      val frameClock = BroadcastFrameClock()
      val viewportState = viewportState()
      val requests = EditorBringIntoViewRequests()
      val session =
        EditorSmoothScrollSession(
          coroutineScope = CoroutineScope(coroutineContext + frameClock),
          viewportState = viewportState,
          bringIntoViewRequests = requests,
        )
      val request = smoothRequest(requests)
      var completionCount = 0

      session.retarget(
        request = request,
        targetY = 580f,
        viewportHeight = 400f,
        maximumScrollY = 1600f,
      ) {
        completionCount += 1
        false
      }
      var frameTimeNanos = pumpFrames(frameClock) { completionCount > 0 }

      assertEquals(580f, viewportState.scrollOffset.y)
      assertEquals(1, completionCount)
      assertTrue(session.active)

      viewportState.scrollToY(targetY = 780f, isAutoScroll = true)
      session.translate(200f)
      session.retarget(
        request = request,
        targetY = 980f,
        viewportHeight = 400f,
        maximumScrollY = 1600f,
      ) {
        completionCount += 1
        true
      }
      frameTimeNanos = pumpFrames(frameClock, frameTimeNanos) { completionCount > 1 }

      assertTrue(frameTimeNanos > 0L)
      assertEquals(980f, viewportState.scrollOffset.y)
      assertEquals(2, completionCount)
      assertFalse(session.active)
    }

  @Test
  fun `retained motion clears when its smooth replacement is discarded`() =
    runTest(StandardTestDispatcher()) {
      val frameClock = BroadcastFrameClock()
      val viewportState = viewportState()
      val requests = EditorBringIntoViewRequests()
      val session =
        EditorSmoothScrollSession(
          coroutineScope = CoroutineScope(coroutineContext + frameClock),
          viewportState = viewportState,
          bringIntoViewRequests = requests,
        )
      val request = smoothRequest(requests)
      var completed = false

      session.retarget(
        request = request,
        targetY = 580f,
        viewportHeight = 400f,
        maximumScrollY = 1600f,
      ) {
        completed = true
        false
      }
      pumpFrames(frameClock) { completed }
      assertTrue(session.active)

      val replacement =
        requests.declare(
          target = EditorBringIntoViewTarget.CurrentSelectionHead,
          policy = EditorBringIntoViewPolicy.Reveal,
          behavior = EditorBringIntoViewBehavior.Smooth,
        )
      runCurrent()
      assertTrue(session.active)

      requests.discard(replacement)
      runCurrent()

      assertFalse(session.active)
    }

  @Test
  fun `running motion stops when its request is discarded`() =
    runTest(StandardTestDispatcher()) {
      val frameClock = BroadcastFrameClock()
      val viewportState = viewportState()
      val requests = EditorBringIntoViewRequests()
      val session =
        EditorSmoothScrollSession(
          coroutineScope = CoroutineScope(coroutineContext + frameClock),
          viewportState = viewportState,
          bringIntoViewRequests = requests,
        )
      val request = smoothRequest(requests)

      session.retarget(
        request = request,
        targetY = 800f,
        viewportHeight = 400f,
        maximumScrollY = 1600f,
      ) {
        true
      }
      requests.discard(request)
      runCurrent()
      val active = session.active
      session.stop()

      assertFalse(active)
    }

  @Test
  fun `motion duration scale slows the custom animator`() =
    runTest(StandardTestDispatcher()) {
      val normalPosition = scrollPositionAfterFrames(durationScale = 1f)
      val slowedPosition = scrollPositionAfterFrames(durationScale = 2f)

      assertTrue(slowedPosition < normalPosition)
    }

  private suspend fun TestScope.scrollPositionAfterFrames(durationScale: Float): Float {
    val frameClock = BroadcastFrameClock()
    val scale = TestMotionDurationScale(durationScale)
    val viewportState = viewportState()
    val requests = EditorBringIntoViewRequests()
    val session =
      EditorSmoothScrollSession(
        coroutineScope = CoroutineScope(coroutineContext + frameClock + scale),
        viewportState = viewportState,
        bringIntoViewRequests = requests,
      )

    session.retarget(
      request = smoothRequest(requests),
      targetY = 800f,
      viewportHeight = 400f,
      maximumScrollY = 1600f,
    ) {
      true
    }

    var frameTimeNanos = 0L
    repeat(10) {
      runCurrent()
      frameTimeNanos += 16_000_000L
      frameClock.sendFrame(frameTimeNanos)
    }
    runCurrent()
    val position = viewportState.scrollOffset.y
    session.stop()
    runCurrent()
    return position
  }

  private fun viewportState(): EditorViewportState =
    EditorViewportState().apply {
      updateMeasuredBounds(
        viewportSize = Size(width = 600f, height = 400f),
        contentSize = Size(width = 600f, height = 2000f),
      )
    }

  private fun smoothRequest(
    requests: EditorBringIntoViewRequests = EditorBringIntoViewRequests()
  ): EditorBringIntoViewRequests.Request =
    requests.requestForVersion(
      target = EditorBringIntoViewTarget.CurrentSelectionHead,
      version = 1L,
      policy = EditorBringIntoViewPolicy.Reveal,
      behavior = EditorBringIntoViewBehavior.Smooth,
    )

  private suspend fun TestScope.pumpFrames(
    frameClock: BroadcastFrameClock,
    initialFrameTimeNanos: Long = 0L,
    done: () -> Boolean,
  ): Long {
    var frameTimeNanos = initialFrameTimeNanos
    repeat(120) {
      runCurrent()
      if (done()) return frameTimeNanos
      frameTimeNanos += 16_000_000L
      frameClock.sendFrame(frameTimeNanos)
    }
    runCurrent()
    return frameTimeNanos
  }

  private class TestMotionDurationScale(override val scaleFactor: Float) : MotionDurationScale
}
