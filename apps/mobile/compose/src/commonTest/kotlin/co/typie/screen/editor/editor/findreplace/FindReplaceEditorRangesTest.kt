package co.typie.screen.editor.editor.findreplace

import co.typie.editor.Editor
import co.typie.editor.FakeFfiEditor
import co.typie.editor.ffi.Affinity
import co.typie.editor.ffi.Message
import co.typie.editor.ffi.Position
import co.typie.editor.ffi.Selection
import co.typie.editor.ffi.TrackedRange
import co.typie.editor.ffi.TrackedRangeOp
import co.typie.editor.ffi.ViewOp
import co.typie.editor.scroll.EditorBringIntoViewBehavior
import co.typie.editor.scroll.EditorBringIntoViewPolicy
import co.typie.editor.scroll.EditorBringIntoViewRequests
import co.typie.editor.scroll.EditorBringIntoViewTarget
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertNotNull
import kotlin.test.assertNull
import kotlinx.coroutines.test.StandardTestDispatcher
import kotlinx.coroutines.test.runTest

class FindReplaceEditorRangesTest {
  private val dispatcher = StandardTestDispatcher()

  @Test
  fun `activating an existing range updates both groups and reveals in one editor revision`() =
    runTest(dispatcher) {
      val bringIntoViewRequests = EditorBringIntoViewRequests()
      val fake = FakeFfiEditor()
      val editor = Editor(fake, this, dispatcher)

      editor.setActiveFindReplaceRangeAndReveal(
        activeId = "match-2",
        currentRanges =
          listOf(
            trackedRange(id = "match-1", group = ACTIVE_SEARCH_MATCH_RANGE_GROUP, start = 0),
            trackedRange(id = "match-2", group = SEARCH_MATCH_RANGE_GROUP, start = 10),
          ),
        bringIntoViewRequests = bringIntoViewRequests,
      )

      assertSingleRequest(
        fake,
        listOf(
          Message.TrackedRange(
            TrackedRangeOp.SetGroup(id = "match-1", group = SEARCH_MATCH_RANGE_GROUP)
          ),
          Message.TrackedRange(
            TrackedRangeOp.SetGroup(id = "match-2", group = ACTIVE_SEARCH_MATCH_RANGE_GROUP)
          ),
          Message.View(ViewOp.ExpandFoldsForTrackedRange(id = "match-2")),
        ),
      )
      val appliedVersion = editor.appliedState.version
      assertNull(bringIntoViewRequests.activateForVersion(version = appliedVersion - 1))
      val reveal = assertNotNull(bringIntoViewRequests.activateForVersion(version = appliedVersion))
      assertEquals(EditorBringIntoViewTarget.TrackedItem("match-2"), reveal.target)
      assertEquals(EditorBringIntoViewPolicy.Reveal, reveal.policy)
      assertEquals(EditorBringIntoViewBehavior.Smooth, reveal.behavior)
    }

  @Test
  fun `replacing ranges with reveal installs final groups and expands the active range atomically`() =
    runTest(dispatcher) {
      val bringIntoViewRequests = EditorBringIntoViewRequests()
      val fake = FakeFfiEditor()
      val editor = Editor(fake, this, dispatcher)
      val first = registration(id = "match-1", start = 0)
      val second = registration(id = "match-2", start = 10)

      editor.setFindReplaceRangesAndReveal(
        items = listOf(first, second),
        activeId = second.id,
        bringIntoViewRequests = bringIntoViewRequests,
      )

      assertSingleRequest(
        fake,
        listOf(
          Message.TrackedRange(TrackedRangeOp.ClearGroup(group = SEARCH_MATCH_RANGE_GROUP)),
          Message.TrackedRange(TrackedRangeOp.ClearGroup(group = ACTIVE_SEARCH_MATCH_RANGE_GROUP)),
          Message.TrackedRange(
            TrackedRangeOp.Add(
              id = first.id,
              group = SEARCH_MATCH_RANGE_GROUP,
              selection = first.selection,
            )
          ),
          Message.TrackedRange(
            TrackedRangeOp.Add(
              id = second.id,
              group = ACTIVE_SEARCH_MATCH_RANGE_GROUP,
              selection = second.selection,
            )
          ),
          Message.View(ViewOp.ExpandFoldsForTrackedRange(id = second.id)),
        ),
      )
      val appliedVersion = editor.appliedState.version
      assertNull(bringIntoViewRequests.activateForVersion(version = appliedVersion - 1))
      val reveal = assertNotNull(bringIntoViewRequests.activateForVersion(version = appliedVersion))
      assertEquals(EditorBringIntoViewTarget.TrackedItem(second.id), reveal.target)
      assertEquals(EditorBringIntoViewPolicy.Reveal, reveal.policy)
      assertEquals(EditorBringIntoViewBehavior.Smooth, reveal.behavior)
    }

  @Test
  fun `refreshing ranges installs the active item directly without expanding folds`() =
    runTest(dispatcher) {
      val fake = FakeFfiEditor()
      val editor = Editor(fake, this, dispatcher)
      val first = registration(id = "match-1", start = 0)
      val second = registration(id = "match-2", start = 10)

      editor.setFindReplaceRanges(items = listOf(first, second), activeId = second.id)

      assertSingleRequest(
        fake,
        listOf(
          Message.TrackedRange(TrackedRangeOp.ClearGroup(group = SEARCH_MATCH_RANGE_GROUP)),
          Message.TrackedRange(TrackedRangeOp.ClearGroup(group = ACTIVE_SEARCH_MATCH_RANGE_GROUP)),
          Message.TrackedRange(
            TrackedRangeOp.Add(
              id = first.id,
              group = SEARCH_MATCH_RANGE_GROUP,
              selection = first.selection,
            )
          ),
          Message.TrackedRange(
            TrackedRangeOp.Add(
              id = second.id,
              group = ACTIVE_SEARCH_MATCH_RANGE_GROUP,
              selection = second.selection,
            )
          ),
        ),
      )
    }

  @Test
  fun `refreshing ranges without an active item installs every item in the normal group`() =
    runTest(dispatcher) {
      val fake = FakeFfiEditor()
      val editor = Editor(fake, this, dispatcher)
      val first = registration(id = "match-1", start = 0)
      val second = registration(id = "match-2", start = 10)

      editor.setFindReplaceRanges(items = listOf(first, second), activeId = null)

      assertSingleRequest(
        fake,
        listOf(
          Message.TrackedRange(TrackedRangeOp.ClearGroup(group = SEARCH_MATCH_RANGE_GROUP)),
          Message.TrackedRange(TrackedRangeOp.ClearGroup(group = ACTIVE_SEARCH_MATCH_RANGE_GROUP)),
          Message.TrackedRange(
            TrackedRangeOp.Add(
              id = first.id,
              group = SEARCH_MATCH_RANGE_GROUP,
              selection = first.selection,
            )
          ),
          Message.TrackedRange(
            TrackedRangeOp.Add(
              id = second.id,
              group = SEARCH_MATCH_RANGE_GROUP,
              selection = second.selection,
            )
          ),
        ),
      )
    }

  @Test
  fun `replacing empty ranges through the reveal path clears both groups without revealing`() =
    runTest(dispatcher) {
      val bringIntoViewRequests = EditorBringIntoViewRequests()
      val fake = FakeFfiEditor()
      val editor = Editor(fake, this, dispatcher)

      editor.setFindReplaceRangesAndReveal(
        items = emptyList(),
        activeId = null,
        bringIntoViewRequests = bringIntoViewRequests,
      )

      assertSingleRequest(
        fake,
        listOf(
          Message.TrackedRange(TrackedRangeOp.ClearGroup(group = SEARCH_MATCH_RANGE_GROUP)),
          Message.TrackedRange(TrackedRangeOp.ClearGroup(group = ACTIVE_SEARCH_MATCH_RANGE_GROUP)),
        ),
      )
      assertNull(bringIntoViewRequests.activateForVersion(version = editor.appliedState.version))
    }

  private fun assertSingleRequest(fake: FakeFfiEditor, expected: List<Message>) {
    assertEquals(1, fake.enqueuedRequests.size)
    assertEquals(expected, fake.enqueuedRequests.single().messages)
  }

  private fun registration(id: String, start: Int): FindReplaceRangeRegistration =
    FindReplaceRangeRegistration(id = id, selection = selection(start))

  private fun trackedRange(id: String, group: String, start: Int): TrackedRange {
    val selection = selection(start)
    return TrackedRange(
      id = id,
      group = group,
      anchor = selection.anchor,
      head = selection.head,
      metadata = "",
      rects = emptyList(),
      text = id,
    )
  }

  private fun selection(start: Int): Selection =
    Selection(
      anchor = Position(node = "text", offset = start, affinity = Affinity.Downstream),
      head = Position(node = "text", offset = start + 5, affinity = Affinity.Downstream),
    )
}
