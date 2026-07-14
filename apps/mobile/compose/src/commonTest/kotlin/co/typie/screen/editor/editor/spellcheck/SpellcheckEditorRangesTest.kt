package co.typie.screen.editor.editor.spellcheck

import co.typie.editor.Editor
import co.typie.editor.FakeFfiEditor
import co.typie.editor.ffi.EditorEvent
import co.typie.editor.ffi.Message
import co.typie.editor.ffi.ProseRangeInstallOutcome
import co.typie.editor.ffi.TrackedRangeOp
import kotlin.test.AfterTest
import kotlin.test.BeforeTest
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertFailsWith
import kotlin.test.assertFalse
import kotlin.test.assertTrue
import kotlinx.coroutines.CompletableDeferred
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.ExperimentalCoroutinesApi
import kotlinx.coroutines.test.StandardTestDispatcher
import kotlinx.coroutines.test.advanceUntilIdle
import kotlinx.coroutines.test.resetMain
import kotlinx.coroutines.test.runTest
import kotlinx.coroutines.test.setMain

private fun rawResult(id: String, start: Int, end: Int) =
  RawSpellcheckResult(
    id = id,
    start = start,
    end = end,
    context = "",
    corrections = emptyList(),
    explanation = "",
  )

@OptIn(ExperimentalCoroutinesApi::class)
class SpellcheckEditorRangesTest {
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
  fun `first result is installed active and remaining results are installed normal`() =
    runTest(dispatcher) {
      lateinit var fake: FakeFfiEditor
      fake =
        FakeFfiEditor(
          onTick = {
            val op = (fake.enqueued.single() as Message.TrackedRange).op
            require(op is TrackedRangeOp.ReplaceGroupsFromProse)
            assertEquals(
              listOf(ACTIVE_SPELLCHECK_RANGE_GROUP, SPELLCHECK_RANGE_GROUP),
              op.ranges.map { it.group },
            )
            listOf(EditorEvent.ProseRangeInstallResult(outcome = ProseRangeInstallOutcome.Applied))
          }
        )
      val editor = Editor(fake, this, dispatcher)

      val result =
        editor.installSpellcheckRangesFromProse(
          expectedText = "hello world",
          items = listOf(rawResult("first", 0, 5), rawResult("second", 6, 11)),
          isCurrent = { true },
        )

      assertEquals(SpellcheckRangeInstallResult.Ready, result)
    }

  @Test
  fun `text mismatch is stale current and invalid ranges fail without partial result`() =
    runTest(dispatcher) {
      lateinit var fake: FakeFfiEditor
      var outcome: ProseRangeInstallOutcome = ProseRangeInstallOutcome.TextMismatch
      fake =
        FakeFfiEditor(
          onTick = {
            val op = (fake.enqueued.last() as Message.TrackedRange).op
            require(op is TrackedRangeOp.ReplaceGroupsFromProse)
            listOf(EditorEvent.ProseRangeInstallResult(outcome))
          }
        )
      val editor = Editor(fake, this, dispatcher)
      val items = listOf(rawResult("first", 0, 5))

      assertEquals(
        SpellcheckRangeInstallResult.StaleCurrent,
        editor.installSpellcheckRangesFromProse("hello", items) { true },
      )

      outcome = ProseRangeInstallOutcome.InvalidRanges(indices = listOf(0))
      val error =
        assertFailsWith<SpellcheckRangeInstallException> {
          editor.installSpellcheckRangesFromProse("hello", items) { true }
        }
      assertEquals(1, error.rawResultCount)
      assertEquals(listOf(FailedSpellcheckRange(index = 0, start = 0, end = 5)), error.failedRanges)
    }

  @Test
  fun `delayed cleanup cannot clear ranges owned by a newer run`() =
    runTest(dispatcher) {
      val request = CompletableDeferred<List<RawSpellcheckResult>>()
      val model = SpellcheckViewModel(request = { _, _ -> request.await() })
      model.runCheck(
        "document",
        { "old" },
        { _, _ -> },
        { _, _, _ -> emptyList() },
        {},
        { _, _ -> },
      )
      advanceUntilIdle()
      val oldRun = requireNotNull(model.pendingCheck).run
      assertTrue(model.cancelCheck(oldRun))
      model.runCheck(
        "document",
        { "new" },
        { _, _ -> },
        { _, _, _ -> emptyList() },
        {},
        { _, _ -> },
      )
      val fake = FakeFfiEditor()
      val editor = Editor(fake, this, dispatcher)

      val cleared = editor.clearSpellcheckRanges(admit = { model.ownsCleanup(oldRun) })

      assertFalse(cleared)
      assertEquals(emptyList(), fake.enqueued)
      assertEquals(0, fake.tickCount)
    }
}
