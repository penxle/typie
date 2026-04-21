package co.typie.screen.editor.editor.state

import androidx.compose.foundation.ScrollState
import androidx.compose.runtime.BroadcastFrameClock
import co.typie.editor.runtime.EditorRuntime
import co.typie.editor.runtime.EditorUiState
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlinx.coroutines.ExperimentalCoroutinesApi
import kotlinx.coroutines.delay
import kotlinx.coroutines.launch
import kotlinx.coroutines.test.advanceTimeBy
import kotlinx.coroutines.test.runCurrent
import kotlinx.coroutines.test.runTest

@OptIn(ExperimentalCoroutinesApi::class)
class EditorScreenStateTest {
  @Test
  fun `prepareToLeaveEditorScene waits for header flush before returning`() = runTest {
    val state = EditorScreenState(scrollState = ScrollState(initial = 0))
    val runtime = EditorRuntime()
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
}
