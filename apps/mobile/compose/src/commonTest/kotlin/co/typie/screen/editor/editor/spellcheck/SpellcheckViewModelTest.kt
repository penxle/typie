package co.typie.screen.editor.editor.spellcheck

import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertNull

class SpellcheckViewModelTest {
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
}
