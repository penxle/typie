package co.typie.editor.sync.ws

import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertNull

class AttachEventReloadTest {
  @Test
  fun reloadEventsDescribeWhetherAReplacementSnapshotAlreadyStarted() {
    assertEquals(true, AttachEvent.SnapshotRestart.replacementSnapshotInFlight())
    assertEquals(true, AttachEvent.ReloadEvent.replacementSnapshotInFlight())
    assertEquals(false, AttachEvent.PermanentErrorEvent("forbidden").replacementSnapshotInFlight())
    assertNull(
      AttachEvent.ChangesetsEvent(
          seq = "1",
          bundles = emptyList(),
          heads = byteArrayOf(),
          durableHeads = byteArrayOf(),
        )
        .replacementSnapshotInFlight()
    )
  }
}
