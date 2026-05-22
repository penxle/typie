package co.typie.screen.editor.editor.viewport

import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertFalse
import kotlin.test.assertTrue
import kotlinx.coroutines.ExperimentalCoroutinesApi
import kotlinx.coroutines.test.advanceTimeBy
import kotlinx.coroutines.test.runCurrent
import kotlinx.coroutines.test.runTest

@OptIn(ExperimentalCoroutinesApi::class)
class EditorDebugWheelZoomSessionTest {
  @Test
  fun `active wheel zoom session ends after burst gap without another scroll event`() = runTest {
    var endCount = 0
    val session =
      EditorDebugWheelZoomSession(
        scope = this,
        timeoutMillis = 56L,
        onSessionEnd = { endCount += 1 },
      )

    session.beginOrKeepAlive()
    advanceTimeBy(55L)
    runCurrent()

    assertTrue(session.active)
    assertEquals(0, endCount)

    advanceTimeBy(1L)
    runCurrent()

    assertFalse(session.active)
    assertEquals(1, endCount)
  }

  @Test
  fun `wheel zoom keep alive restarts the burst gap timeout`() = runTest {
    var endCount = 0
    val session =
      EditorDebugWheelZoomSession(
        scope = this,
        timeoutMillis = 56L,
        onSessionEnd = { endCount += 1 },
      )

    session.beginOrKeepAlive()
    advanceTimeBy(40L)
    session.beginOrKeepAlive()
    advanceTimeBy(40L)
    runCurrent()

    assertTrue(session.active)
    assertEquals(0, endCount)

    advanceTimeBy(16L)
    runCurrent()

    assertFalse(session.active)
    assertEquals(1, endCount)
  }
}
