package co.typie.screen.editor.editor

import co.typie.editor.ffi.Affinity
import co.typie.editor.ffi.Position
import co.typie.editor.ffi.TrackedRangeEndpoints
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertNull

class TrackedRangeMembershipTest {
  private val position = Position(node = "paragraph", offset = 1, affinity = Affinity.Downstream)

  private fun range(id: String, group: String) =
    TrackedRangeEndpoints(id = id, group = group, anchor = position, head = position)

  @Test
  fun eligibleActiveMemberWinsWithoutChangingCoreFallbackOrder() {
    val members = listOf(range("before", "comment"), range("after", "comment-active"))

    assertEquals(
      "after",
      members
        .selectTrackedRangeMember(
          allowedGroups = setOf("comment", "comment-active"),
          activeId = "after",
        )
        ?.id,
    )
    assertEquals(
      "before",
      members
        .selectTrackedRangeMember(
          allowedGroups = setOf("comment", "comment-active"),
          activeId = null,
        )
        ?.id,
    )
  }

  @Test
  fun excludedOrUnownedActiveIdFallsBackToFirstEligibleMember() {
    val members =
      listOf(
        range("temporary", "__comment_compose__"),
        range("missing-result", "spellcheck-active"),
        range("owned-result", "spellcheck"),
      )

    assertEquals(
      "owned-result",
      members
        .selectTrackedRangeMember(
          allowedGroups = setOf("spellcheck", "spellcheck-active"),
          activeId = "missing-result",
          ownedIds = setOf("owned-result"),
        )
        ?.id,
    )
    assertNull(
      members.selectTrackedRangeMember(
        allowedGroups = setOf("comment", "comment-active"),
        activeId = "temporary",
      )
    )
    assertNull(
      listOf(range("orphan", "comment"))
        .selectTrackedRangeMember(
          allowedGroups = setOf("comment", "comment-active"),
          activeId = "orphan",
          ownedIds = emptySet(),
        )
    )
  }

  @Test
  fun membershipIdsIgnoreNormalActiveGroupProjection() {
    val groups = setOf("comment", "comment-active")

    val before =
      listOf(range("a", "comment"), range("b", "comment-active")).trackedRangeMembershipIds(groups)
    val after =
      listOf(range("a", "comment-active"), range("b", "comment")).trackedRangeMembershipIds(groups)

    assertEquals(before, after)
  }
}
