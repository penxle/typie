package co.typie.screen.editor.editor.aifeedback

import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertFalse
import kotlin.test.assertNull
import kotlin.test.assertTrue

class AiFeedbackViewModelTest {
  @Test
  fun `prepare analysis clears previous run and enters loading state`() {
    val model = AiFeedbackViewModel()
    model.enterMode()
    model.complete()
    model.appendResult(result(id = "old"))

    model.prepareAnalysis("new text")

    assertTrue(model.active)
    assertTrue(model.loading)
    assertFalse(model.hasCompleted)
    assertEquals("new text", model.pendingAnalysisText)
    assertEquals(emptyList(), model.results)
    assertNull(model.currentCardId)
    assertNull(model.activeRangeId)
    assertNull(model.progress)
  }

  @Test
  fun `complete with zero results keeps completed badge state`() {
    val model = AiFeedbackViewModel()
    model.enterMode()
    model.prepareAnalysis("text")
    model.updateProgress(AiFeedbackProgress(current = 0, total = 1, phase = "analyzing"))

    model.complete()

    assertFalse(model.loading)
    assertTrue(model.hasCompleted)
    assertNull(model.pendingAnalysisText)
    assertNull(model.progress)
    assertEquals(0, model.resultCount)
  }

  @Test
  fun `streamed first result becomes current and active`() {
    val model = AiFeedbackViewModel()
    model.prepareAnalysis("text")

    model.appendResult(result(id = "first"))
    model.appendResult(result(id = "second"))

    assertEquals(listOf("first", "second"), model.results.map { it.id })
    assertEquals("first", model.currentCardId)
    assertEquals("first", model.activeRangeId)
  }

  @Test
  fun `removing active result selects the next result`() {
    val model = AiFeedbackViewModel()
    model.prepareAnalysis("text")
    model.appendResult(result(id = "first"))
    model.appendResult(result(id = "second"))

    val replacement = model.remove("first", activateReplacement = true)

    assertEquals("second", replacement)
    assertEquals("second", model.currentCardId)
    assertEquals("second", model.activeRangeId)
    assertEquals(listOf("second"), model.results.map { it.id })
  }

  @Test
  fun `cancel analysis clears progress and results without completed badge`() {
    val model = AiFeedbackViewModel()
    model.enterMode()
    model.prepareAnalysis("text")
    model.updateProgress(AiFeedbackProgress(current = 0, total = 1, phase = "analyzing"))
    model.appendResult(result(id = "first"))

    model.cancelAnalysis()

    assertTrue(model.active)
    assertFalse(model.loading)
    assertFalse(model.hasCompleted)
    assertNull(model.pendingAnalysisText)
    assertNull(model.progress)
    assertEquals(emptyList(), model.results)
    assertNull(model.currentCardId)
    assertNull(model.activeRangeId)
  }

  @Test
  fun `exit mode clears analysis state and invalidates current run`() {
    val model = AiFeedbackViewModel()
    model.enterMode()
    val runId = model.prepareAnalysis("text")
    model.updateProgress(AiFeedbackProgress(current = 0, total = 1, phase = "analyzing"))
    model.appendResult(result(id = "first"))

    model.exitMode()

    assertFalse(model.active)
    assertFalse(model.loading)
    assertFalse(model.hasCompleted)
    assertFalse(model.isCurrentAnalysisRun(runId))
    assertNull(model.pendingAnalysisText)
    assertNull(model.progress)
    assertEquals(emptyList(), model.results)
    assertNull(model.currentCardId)
    assertNull(model.activeRangeId)
  }

  @Test
  fun `starting a new analysis invalidates previous run`() {
    val model = AiFeedbackViewModel()

    val firstRun = model.prepareAnalysis("same text")
    val secondRun = model.prepareAnalysis("same text")

    assertFalse(model.isCurrentAnalysisRun(firstRun))
    assertTrue(model.isCurrentAnalysisRun(secondRun))

    model.cancelAnalysis()

    assertFalse(model.isCurrentAnalysisRun(secondRun))
  }

  @Test
  fun `cleanup missing ranges removes stale results and clears missing active range`() {
    val model = AiFeedbackViewModel()
    model.prepareAnalysis("text")
    model.appendResult(result(id = "first"))
    model.appendResult(result(id = "second"))
    model.activate("second")

    val removedIds = model.cleanupMissingRanges(liveIds = setOf("first"))

    assertEquals(setOf("second"), removedIds)
    assertEquals(listOf("first"), model.results.map { it.id })
    assertEquals("first", model.currentCardId)
    assertNull(model.activeRangeId)
  }

  @Test
  fun `activating unknown result clears active range but keeps current card`() {
    val model = AiFeedbackViewModel()
    model.appendResult(result(id = "first"))

    model.activate("missing")

    assertEquals("first", model.currentCardId)
    assertNull(model.activeRangeId)
  }

  private fun result(id: String): AiFeedbackResult =
    AiFeedbackResult(
      id = id,
      startText = "start",
      endText = "end",
      feedback = "feedback",
      category = null,
    )
}
