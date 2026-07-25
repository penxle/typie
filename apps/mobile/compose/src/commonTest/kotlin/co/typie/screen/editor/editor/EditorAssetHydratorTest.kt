package co.typie.screen.editor.editor

import co.typie.editor.external.EditorAssetResolution
import co.typie.editor.external.EditorExternalElementState
import co.typie.editor.sync.ws.WsAssetStateEntry
import co.typie.editor.sync.ws.WsPendingMeta
import co.typie.editor.sync.ws.WsReadyAsset
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertNull
import kotlin.test.assertTrue
import kotlinx.coroutines.ExperimentalCoroutinesApi
import kotlinx.coroutines.test.TestScope
import kotlinx.coroutines.test.advanceTimeBy
import kotlinx.coroutines.test.runCurrent
import kotlinx.coroutines.test.runTest

@OptIn(ExperimentalCoroutinesApi::class)
class EditorAssetHydratorTest {
  private class Harness(
    testScope: TestScope,
    basePollMs: Long = 1_000,
    maxPollMs: Long = 4_000,
    debounceMs: Long = 10,
  ) {
    val state = EditorExternalElementState()
    val pulls = mutableListOf<Pair<String, List<String>>>()
    val sync =
      EditorAssetHydrator(
        scope = testScope.backgroundScope,
        state = state,
        pull = { requestId, ids -> pulls += requestId to ids },
        basePollMs = basePollMs,
        maxPollMs = maxPollMs,
        debounceMs = debounceMs,
      )

    fun idsOf(): List<List<String>> = pulls.map { it.second }

    fun lastRequestId(): String = pulls.lastOrNull()?.first.orEmpty()
  }

  private fun ready(id: String): WsAssetStateEntry =
    WsAssetStateEntry(
      id = id,
      state = "ready",
      asset =
        WsReadyAsset(
          type = "image",
          id = id,
          url = "https://cdn/$id",
          width = 10,
          height = 20,
          placeholder = null,
        ),
    )

  private fun pending(id: String): WsAssetStateEntry =
    WsAssetStateEntry(
      id = id,
      state = "pending",
      meta = WsPendingMeta(kind = "image", name = "$id.png", size = 100),
    )

  private fun missing(id: String): WsAssetStateEntry = WsAssetStateEntry(id = id, state = "missing")

  @Test
  fun pullsOnlyUnresolvedIdsAndCoalescesBurstIntoOneRequest() = runTest {
    val h = Harness(this)

    h.sync.update(listOf("a", "b"))
    h.sync.update(listOf("a", "b", "c"))
    advanceTimeBy(10)
    runCurrent()

    assertEquals(listOf(listOf("a", "b", "c")), h.idsOf())

    h.sync.receive(h.lastRequestId(), listOf(ready("a"), pending("b"), missing("c")), true)
    h.pulls.clear()

    h.sync.update(listOf("a", "b", "c", "d"))
    advanceTimeBy(10)
    runCurrent()

    assertEquals(listOf(listOf("d")), h.idsOf())
  }

  @Test
  fun doesNotRepullIdWhoseResponseHasNotArrivedYet() = runTest {
    val h = Harness(this)

    h.sync.update(listOf("a"))
    advanceTimeBy(10)
    runCurrent()
    h.sync.update(listOf("a"))
    advanceTimeBy(10)
    runCurrent()

    assertEquals(listOf(listOf("a")), h.idsOf())
  }

  @Test
  fun discardsEntriesUnderSupersededRequestId() = runTest {
    val h = Harness(this)

    h.sync.update(listOf("a"))
    advanceTimeBy(10)
    runCurrent()
    val stale = h.lastRequestId()

    h.sync.invalidate(listOf("a"))
    advanceTimeBy(10)
    runCurrent()
    val fresh = h.lastRequestId()

    assertTrue(fresh != stale)

    h.sync.receive(fresh, listOf(missing("a")), true)
    h.sync.receive(stale, listOf(pending("a")), true)

    assertEquals(EditorAssetResolution.Missing, h.state.resolutions["a"])
  }

  @Test
  fun clearsAwaitingMarkWhenFinalFrameCarriesNoEntryForRequestedId() = runTest {
    val h = Harness(this)

    h.sync.update(listOf("a"))
    advanceTimeBy(10)
    runCurrent()
    val requestId = h.lastRequestId()
    h.pulls.clear()

    h.sync.receive(requestId, emptyList(), true)

    h.sync.update(listOf("a"))
    advanceTimeBy(10)
    runCurrent()

    assertEquals(listOf(listOf("a")), h.idsOf())
  }

  @Test
  fun accumulatesChunkedResponseAcrossFramesSharingOneRequestId() = runTest {
    val h = Harness(this)

    h.sync.update(listOf("a", "b"))
    advanceTimeBy(10)
    runCurrent()
    val requestId = h.lastRequestId()

    h.sync.receive(requestId, listOf(ready("a")), false)
    assertEquals("a", h.state.images.assets["a"]?.id)
    assertNull(h.state.resolutions["b"])

    h.sync.receive(requestId, listOf(pending("b")), true)
    assertTrue(h.state.resolutions["b"] is EditorAssetResolution.Pending)
  }

  @Test
  fun invalidatesOnlyReferencedIds() = runTest {
    val h = Harness(this)

    h.sync.update(listOf("a"))
    advanceTimeBy(10)
    runCurrent()
    h.pulls.clear()

    h.sync.invalidate(listOf("a", "gone"))
    advanceTimeBy(10)
    runCurrent()

    assertEquals(listOf(listOf("a")), h.idsOf())

    h.pulls.clear()
    h.sync.invalidate(listOf("gone"))
    advanceTimeBy(10)
    runCurrent()

    assertTrue(h.pulls.isEmpty())
  }

  @Test
  fun dropsQueuedIdsThatLeaveReferenceSetBeforeDebounceFires() = runTest {
    val h = Harness(this)

    h.sync.update(listOf("a", "b"))
    advanceTimeBy(10)
    runCurrent()
    h.sync.receive(h.lastRequestId(), listOf(missing("a"), missing("b")), true)
    h.pulls.clear()

    h.sync.invalidate(listOf("a", "b"))
    h.sync.update(listOf("a"))
    advanceTimeBy(10)
    runCurrent()

    assertEquals(listOf(listOf("a")), h.idsOf())
  }

  @Test
  fun coalescesInvalidationStormIntoSinglePull() = runTest {
    val h = Harness(this)

    h.sync.update(listOf("a", "b", "c"))
    advanceTimeBy(10)
    runCurrent()
    h.sync.receive(h.lastRequestId(), listOf(pending("a"), pending("b"), pending("c")), true)
    h.pulls.clear()

    h.sync.invalidate(listOf("a"))
    h.sync.invalidate(listOf("b"))
    h.sync.invalidate(listOf("a", "c"))
    advanceTimeBy(10)
    runCurrent()

    assertEquals(listOf(listOf("a", "b", "c")), h.idsOf())
  }

  @Test
  fun completedAssetWinsOverLaterPendingOrMissingFrames() = runTest {
    val h = Harness(this)

    h.sync.update(listOf("a"))
    advanceTimeBy(10)
    runCurrent()
    h.sync.receive(h.lastRequestId(), listOf(pending("a")), true)
    assertTrue(h.state.resolutions["a"] is EditorAssetResolution.Pending)

    h.sync.invalidate(listOf("a"))
    advanceTimeBy(10)
    runCurrent()
    h.sync.receive(h.lastRequestId(), listOf(ready("a")), true)
    assertEquals("a", h.state.images.assets["a"]?.id)
    assertNull(h.state.resolutions["a"])

    h.sync.invalidate(listOf("a"))
    advanceTimeBy(10)
    runCurrent()
    h.sync.receive(h.lastRequestId(), listOf(missing("a")), true)

    assertEquals("a", h.state.images.assets["a"]?.id)
    assertNull(h.state.resolutions["a"])

    h.pulls.clear()
    advanceTimeBy(60_000)
    runCurrent()

    assertTrue(h.pulls.isEmpty())
  }

  @Test
  fun rePullsPendingIdAfterPollIntervalAndAppliesSilentExpiry() = runTest {
    val h = Harness(this)

    h.sync.update(listOf("a"))
    advanceTimeBy(10)
    runCurrent()
    h.sync.receive(h.lastRequestId(), listOf(pending("a")), true)
    h.pulls.clear()

    advanceTimeBy(1_000)
    runCurrent()

    assertEquals(listOf(listOf("a")), h.idsOf())

    h.sync.receive(h.lastRequestId(), listOf(missing("a")), true)

    assertEquals(EditorAssetResolution.Missing, h.state.resolutions["a"])
  }

  @Test
  fun keepsPollingIdsCachedAsMissingSoDroppedInvalidationStillConverges() = runTest {
    val h = Harness(this)

    h.sync.update(listOf("a"))
    advanceTimeBy(10)
    runCurrent()
    h.sync.receive(h.lastRequestId(), listOf(missing("a")), true)
    h.pulls.clear()

    advanceTimeBy(1_000)
    runCurrent()

    assertEquals(listOf(listOf("a")), h.idsOf())

    h.sync.receive(h.lastRequestId(), listOf(ready("a")), true)

    assertEquals("a", h.state.images.assets["a"]?.id)
  }

  @Test
  fun growsPollIntervalUpToCapAndResetsOnStateChange() = runTest {
    val h = Harness(this)

    h.sync.update(listOf("a"))
    advanceTimeBy(10)
    runCurrent()
    h.sync.receive(h.lastRequestId(), listOf(missing("a")), true)
    h.pulls.clear()

    advanceTimeBy(1_000)
    runCurrent()
    assertEquals(1, h.pulls.size)
    h.sync.receive(h.lastRequestId(), listOf(missing("a")), true)

    advanceTimeBy(1_000)
    runCurrent()
    assertEquals(1, h.pulls.size)
    advanceTimeBy(1_000)
    runCurrent()
    assertEquals(2, h.pulls.size)
    h.sync.receive(h.lastRequestId(), listOf(missing("a")), true)

    advanceTimeBy(4_000)
    runCurrent()
    assertEquals(3, h.pulls.size)
    h.sync.receive(h.lastRequestId(), listOf(missing("a")), true)

    advanceTimeBy(4_000)
    runCurrent()
    assertEquals(4, h.pulls.size)

    h.sync.receive(h.lastRequestId(), listOf(pending("a")), true)
    advanceTimeBy(1_000)
    runCurrent()
    assertEquals(5, h.pulls.size)
  }

  @Test
  fun resetsPollIntervalWhenReturningToForeground() = runTest {
    val h = Harness(this)

    h.sync.update(listOf("a"))
    advanceTimeBy(10)
    runCurrent()
    h.sync.receive(h.lastRequestId(), listOf(missing("a")), true)

    advanceTimeBy(1_000)
    runCurrent()
    h.sync.receive(h.lastRequestId(), listOf(missing("a")), true)
    advanceTimeBy(2_000)
    runCurrent()
    h.sync.receive(h.lastRequestId(), listOf(missing("a")), true)
    h.pulls.clear()

    h.sync.repullReferenced(listOf("a"))
    runCurrent()
    assertEquals(1, h.pulls.size)

    advanceTimeBy(1_000)
    runCurrent()
    assertEquals(2, h.pulls.size)
  }

  @Test
  fun repullsEveryReferencedNonReadyIdAtOnceAndSkipsReadyOnes() = runTest {
    val h = Harness(this)

    h.sync.update(listOf("a", "b", "c"))
    advanceTimeBy(10)
    runCurrent()
    h.sync.receive(h.lastRequestId(), listOf(ready("a"), pending("b"), missing("c")), true)
    h.pulls.clear()

    h.sync.repullReferenced(listOf("a", "b", "c"))
    runCurrent()

    assertEquals(listOf(listOf("b", "c")), h.idsOf())
  }

  @Test
  fun dropsIdsThatLeftReferenceSetFromPollingAndFromResponses() = runTest {
    val h = Harness(this)

    h.sync.update(listOf("a", "b"))
    advanceTimeBy(10)
    runCurrent()
    val requestId = h.lastRequestId()
    h.sync.update(listOf("a"))
    h.pulls.clear()

    h.sync.receive(requestId, listOf(missing("a"), ready("b")), true)

    assertEquals(EditorAssetResolution.Missing, h.state.resolutions["a"])
    assertNull(h.state.images.assets["b"])

    advanceTimeBy(1_000)
    runCurrent()

    assertEquals(listOf(listOf("a")), h.idsOf())
  }

  @Test
  fun splitsPullOverTheWireLimitIntoBoundedRequests() = runTest {
    val h = Harness(this)
    val ids = (0 until 250).map { index -> "id-$index" }

    h.sync.update(ids)
    advanceTimeBy(10)
    runCurrent()

    assertEquals(listOf(100, 100, 50), h.idsOf().map(List<String>::size))
    assertEquals(3, h.pulls.map { it.first }.toSet().size)
    assertEquals(ids, h.idsOf().flatten())
  }

  @Test
  fun stopsEveryTimerAndIgnoresFurtherCallsAfterDispose() = runTest {
    val h = Harness(this)

    h.sync.update(listOf("a"))
    advanceTimeBy(10)
    runCurrent()
    h.sync.receive(h.lastRequestId(), listOf(missing("a")), true)
    h.pulls.clear()

    h.sync.dispose()

    h.sync.update(listOf("a", "b"))
    h.sync.invalidate(listOf("a"))
    h.sync.repullReferenced(listOf("a"))
    advanceTimeBy(60_000)
    runCurrent()

    assertTrue(h.pulls.isEmpty())
  }
}
