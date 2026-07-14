package co.typie.screen.editor.editor.spellcheck

import kotlin.test.AfterTest
import kotlin.test.BeforeTest
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertFalse
import kotlin.test.assertNull
import kotlin.test.assertTrue
import kotlinx.coroutines.CompletableDeferred
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.ExperimentalCoroutinesApi
import kotlinx.coroutines.test.StandardTestDispatcher
import kotlinx.coroutines.test.advanceUntilIdle
import kotlinx.coroutines.test.resetMain
import kotlinx.coroutines.test.runTest
import kotlinx.coroutines.test.setMain

@OptIn(ExperimentalCoroutinesApi::class)
class SpellcheckViewModelTest {
  private val dispatcher = StandardTestDispatcher()

  @BeforeTest
  fun setUp() {
    Dispatchers.setMain(dispatcher)
  }

  @AfterTest
  fun tearDown() {
    Dispatchers.resetMain()
  }

  @Test
  fun `exit mode clears check state`() =
    runTest(dispatcher) {
      val request = CompletableDeferred<List<RawSpellcheckResult>>()
      val model = SpellcheckViewModel(request = { _, _ -> request.await() })
      model.enterMode()
      model.runCheck(
        "document",
        { "source text" },
        { _, _ -> },
        { _, _, _ -> emptyList() },
        {},
        { _, _ -> },
      )
      advanceUntilIdle()

      model.exitMode()

      assertFalse(model.active)
      assertFalse(model.loading)
      assertFalse(model.ready)
      assertTrue(model.hasNoActiveRun())
      assertNull(model.pendingCheck)
      assertEquals(emptyList(), model.results)
      assertNull(model.currentCardId)
      assertNull(model.activeRangeId)
    }

  @Test
  fun `ready results publish while loader is still loading`() =
    runTest(dispatcher) {
      val model =
        SpellcheckViewModel(request = { _, _ -> listOf(raw(id = "first", context = "context")) })
      var loadingDuringReady = false

      model.runCheck(
        documentId = "document",
        sourceText = { "context" },
        beforeRequest = { _, _ -> },
        prepareResults = { raw, _, _ -> raw.map { it.toResult() } },
        onReady = { loadingDuringReady = model.loading },
        onError = { _, _ -> },
      )
      advanceUntilIdle()

      assertTrue(loadingDuringReady)
      assertEquals(listOf("first"), model.results.map { it.id })
      assertTrue(model.ready)
      assertFalse(model.loading)
      assertNull(model.pendingCheck)
    }

  @Test
  fun `pending check can be cancelled only by its current run`() =
    runTest(dispatcher) {
      val request = CompletableDeferred<List<RawSpellcheckResult>>()
      val model = SpellcheckViewModel(request = { _, _ -> request.await() })
      model.runCheck(
        "document",
        { "source" },
        { _, _ -> },
        { _, _, _ -> emptyList() },
        {},
        { _, _ -> },
      )
      advanceUntilIdle()
      val pending = requireNotNull(model.pendingCheck)

      assertTrue(model.cancelCheck(pending.run))
      assertFalse(model.cancelCheck(pending.run))
      assertTrue(model.ownsCleanup(pending.run))
      assertNull(model.pendingCheck)
      assertFalse(model.loading)
    }

  @Test
  fun `ready callback failure clears published model and becomes loader error`() =
    runTest(dispatcher) {
      val failure = IllegalStateException("ready")
      val errors = mutableListOf<Throwable>()
      var cleanupWasCurrent = false
      val model = SpellcheckViewModel(request = { _, _ -> listOf(raw("first", "context")) })

      model.runCheck(
        "document",
        { "context" },
        { _, _ -> },
        { raw, _, _ -> raw.map { it.toResult() } },
        { throw failure },
        { error, run ->
          cleanupWasCurrent = model.ownsCleanup(run)
          errors.add(error)
        },
      )
      advanceUntilIdle()

      assertEquals(failure, model.error)
      assertEquals(listOf<Throwable>(failure), errors)
      assertTrue(cleanupWasCurrent)
      assertEquals(emptyList(), model.results)
      assertNull(model.pendingCheck)
    }

  @Test
  fun `loading check does not start a second run`() =
    runTest(dispatcher) {
      val request = CompletableDeferred<List<RawSpellcheckResult>>()
      val model = SpellcheckViewModel(request = { _, _ -> request.await() })
      var secondSourceCaptured = false

      model.runCheck(
        "document",
        { "first" },
        { _, _ -> },
        { _, _, _ -> emptyList() },
        {},
        { _, _ -> },
      )
      advanceUntilIdle()
      val firstRun = requireNotNull(model.pendingCheck).run

      model.runCheck(
        "document",
        {
          secondSourceCaptured = true
          "second"
        },
        { _, _ -> },
        { _, _, _ -> emptyList() },
        {},
        { _, _ -> },
      )
      advanceUntilIdle()

      assertTrue(model.isCurrent(firstRun))
      assertFalse(secondSourceCaptured)
    }

  @Test
  fun `stale current result removed after direct edit does not activate replacement`() {
    val model = SpellcheckViewModel()
    model.replaceResults(
      listOf(result(id = "first", context = "first"), result(id = "second", context = "second"))
    )
    model.activate(null)

    val cleanup = model.cleanupStale(mapOf("second" to "second"))

    assertEquals(setOf("first"), cleanup)
    assertEquals("second", model.currentCardId)
    assertNull(model.activeRangeId)
  }

  private fun result(id: String, context: String): SpellcheckResult =
    SpellcheckResult(id = id, context = context, corrections = emptyList(), explanation = "")

  private fun raw(id: String, context: String): RawSpellcheckResult =
    RawSpellcheckResult(
      id = id,
      start = 0,
      end = context.length,
      context = context,
      corrections = emptyList(),
      explanation = "",
    )

  private fun RawSpellcheckResult.toResult(): SpellcheckResult =
    SpellcheckResult(
      id = id,
      context = context,
      corrections = corrections,
      explanation = explanation,
    )
}
