package co.typie.screen.editor.editor.state

import androidx.compose.runtime.BroadcastFrameClock
import androidx.compose.ui.geometry.Size
import co.typie.editor.Editor
import co.typie.editor.FakeFfiEditor
import co.typie.editor.runtime.EditorRuntime
import co.typie.editor.runtime.EditorUiState
import co.typie.editor.viewport.EditorViewportState
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertFalse
import kotlin.test.assertSame
import kotlin.test.assertTrue
import kotlinx.coroutines.ExperimentalCoroutinesApi
import kotlinx.coroutines.delay
import kotlinx.coroutines.launch
import kotlinx.coroutines.test.StandardTestDispatcher
import kotlinx.coroutines.test.advanceTimeBy
import kotlinx.coroutines.test.advanceUntilIdle
import kotlinx.coroutines.test.runCurrent
import kotlinx.coroutines.test.runTest

@OptIn(ExperimentalCoroutinesApi::class)
class EditorScreenStateTest {
  @Test
  fun `visible area uses max of editor input and sub pane bottom occlusion`() {
    val viewportState = EditorViewportState()
    viewportState.updateMeasuredBounds(viewportSize = Size(400f, 800f), contentSize = Size.Zero)
    val state = EditorScreenState(viewportState = viewportState)

    val visibleArea =
      state.resolveVisibleArea(
        topInset = 24f,
        rawBottomSafeInset = 20f,
        rawEditorInputBottomInset = 120f,
        rawSubPaneBottomInset = 360f,
      )

    assertEquals(360f, visibleArea.bottomOcclusion)
    assertEquals(416f, visibleArea.visibleBodySize.height)
  }

  @Test
  fun `non foreground scene ignores sub pane bottom occlusion`() = runTest {
    val viewportState = EditorViewportState()
    viewportState.updateMeasuredBounds(viewportSize = Size(400f, 800f), contentSize = Size.Zero)
    val state = EditorScreenState(viewportState = viewportState)
    val runtime = EditorRuntime(uiScope = this)
    val uiState = EditorUiState()

    state.updateSceneForeground(isForeground = false, runtime = runtime, uiState = uiState)

    val visibleArea =
      state.resolveVisibleArea(
        topInset = 24f,
        rawBottomSafeInset = 20f,
        rawEditorInputBottomInset = 120f,
        rawSubPaneBottomInset = 360f,
      )

    assertEquals(0f, visibleArea.bottomOcclusion)
    assertEquals(776f, visibleArea.visibleBodySize.height)
  }

  @Test
  fun `leaving the foreground contains an editor admission failure at the scene callback`() =
    runTest {
      val failure = IllegalStateException("admission failed")
      val runtime = EditorRuntime(uiScope = this)
      val editor =
        Editor(
          inner = FakeFfiEditor(beforeEnqueueRequest = { throw failure }),
          scope = this,
          dispatcher = StandardTestDispatcher(testScheduler),
          onError = { activeEditor, error -> runtime.fail(activeEditor, error) },
        )
      runtime.attach(editor)
      val state = EditorScreenState(viewportState = EditorViewportState())

      state.updateSceneForeground(
        isForeground = false,
        runtime = runtime,
        uiState = EditorUiState(),
      )
      advanceUntilIdle()

      assertFalse(state.sceneInForeground)
      assertSame(failure, runtime.failure)
    }

  @Test
  fun `prepareToLeaveEditorScene waits for header flush before returning`() = runTest {
    val state = EditorScreenState(viewportState = EditorViewportState())
    val runtime = EditorRuntime(uiScope = this)
    val uiState = EditorUiState()
    val events = mutableListOf<String>()
    val frameClock = BroadcastFrameClock()

    val leaveJob =
      launch(frameClock) {
        state.prepareToLeaveEditorScene(runtime = runtime, uiState = uiState) {
          events += "flush-start"
          delay(10)
          events += "flush-end"
        }
        events += "returned"
      }

    runCurrent()
    assertEquals(listOf("flush-start"), events)

    advanceTimeBy(10)
    runCurrent()
    assertEquals(listOf("flush-start", "flush-end"), events)

    frameClock.sendFrame(0L)
    runCurrent()
    assertEquals(listOf("flush-start", "flush-end", "returned"), events)

    leaveJob.join()
  }

  @Test
  fun `preparing to leave contains an editor admission failure and still flushes the header`() =
    runTest {
      val failure = IllegalStateException("admission failed")
      val runtime = EditorRuntime(uiScope = this)
      val editor =
        Editor(
          inner = FakeFfiEditor(beforeEnqueueRequest = { throw failure }),
          scope = this,
          dispatcher = StandardTestDispatcher(testScheduler),
          onError = { activeEditor, error -> runtime.fail(activeEditor, error) },
        )
      runtime.attach(editor)
      val state = EditorScreenState(viewportState = EditorViewportState())
      val frameClock = BroadcastFrameClock()
      var flushed = false

      val leaveJob =
        launch(frameClock) {
          state.prepareToLeaveEditorScene(
            runtime = runtime,
            uiState = EditorUiState(),
            flushDrafts = { flushed = true },
          )
        }
      runCurrent()
      frameClock.sendFrame(0L)
      advanceUntilIdle()

      assertTrue(flushed)
      assertSame(failure, runtime.failure)
      leaveJob.join()
    }
}
