package co.typie.editor.sync

import co.typie.editor.sync.ws.SyncWsException
import kotlin.test.Test
import kotlin.test.assertFalse
import kotlin.test.assertTrue

class SyncErrorClassificationTest {
  @Test
  fun wrappedPermanentErrorsAreUnwrapped() {
    val causeWrapped = RuntimeException("wrapper", SyncWsException("forbidden", true))
    assertTrue(isPermanentSyncError(causeWrapped))

    val suppressedWrapped = RuntimeException("outer")
    suppressedWrapped.addSuppressed(SyncWsException("forbidden", true))
    assertTrue(isPermanentSyncError(suppressedWrapped))

    val wrappedTransient = RuntimeException("wrapper", SyncWsException("internal", false))
    assertFalse(isPermanentSyncError(wrappedTransient))
  }

  @Test
  fun mutuallySuppressedCycleTerminates() {
    val a = RuntimeException("a")
    val b = RuntimeException("b")
    a.addSuppressed(b)
    b.addSuppressed(a)
    assertFalse(isPermanentSyncError(a))

    val c = RuntimeException("c")
    val d = RuntimeException("d", SyncWsException("forbidden", true))
    c.addSuppressed(d)
    d.addSuppressed(c)
    assertTrue(isPermanentSyncError(c))
  }
}
