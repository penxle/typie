package co.typie.screen.document.document

import co.typie.domain.goal.pickGoalSource
import co.typie.domain.goal.toEntityGoalData
import co.typie.graphql.DocumentScreen_Query
import co.typie.graphql.fragment.EntityGoalFields_goal
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertNull
import kotlin.test.assertTrue
import kotlin.time.Instant
import kotlinx.datetime.LocalDate

class DocumentGoalMappingTest {
  private fun goalFields(
    id: String,
    dueAt: Instant?,
    createdAt: Instant,
    targetCharacterCount: Int,
  ) =
    EntityGoalFields_goal(
      __typename = "EntityGoal",
      id = id,
      targetCharacterCount = targetCharacterCount,
      dueAt = dueAt,
      createdAt = createdAt,
    )

  private fun ownGoal(dueAt: Instant?, createdAt: Instant, targetCharacterCount: Int = 10_000) =
    DocumentScreen_Query.Goal(
      __typename = "EntityGoal",
      entityGoalFields_goal =
        goalFields(
          id = "goal-own",
          dueAt = dueAt,
          createdAt = createdAt,
          targetCharacterCount = targetCharacterCount,
        ),
    )

  private fun ancestorGoal(
    id: String = "goal-ancestor",
    dueAt: Instant? = null,
    createdAt: Instant = Instant.parse("2026-07-01T00:00:00Z"),
    targetCharacterCount: Int = 50_000,
  ) =
    DocumentScreen_Query.Goal1(
      __typename = "EntityGoal",
      entityGoalFields_goal =
        goalFields(
          id = id,
          dueAt = dueAt,
          createdAt = createdAt,
          targetCharacterCount = targetCharacterCount,
        ),
    )

  private fun folderAncestor(
    id: String,
    goal: DocumentScreen_Query.Goal1?,
    characterCount: Int,
  ) =
    DocumentScreen_Query.Ancestor(
      __typename = "Entity",
      id = id,
      goal = goal,
      node =
        DocumentScreen_Query.Node(
          __typename = "Folder",
          onFolder =
            DocumentScreen_Query.OnFolder(id = "$id-node", characterCount = characterCount),
        ),
    )

  private fun documentAncestor(id: String, goal: DocumentScreen_Query.Goal1?) =
    DocumentScreen_Query.Ancestor(
      __typename = "Entity",
      id = id,
      goal = goal,
      node = DocumentScreen_Query.Node(__typename = "Document", onFolder = null),
    )

  @Test
  fun ancestorGoalDatesUseKstCalendarDay() {
    val candidate =
      folderAncestor(
          id = "folder-1",
          goal =
            ancestorGoal(
              dueAt = Instant.parse("2026-08-05T15:00:00Z"),
              createdAt = Instant.parse("2026-07-31T15:00:00Z"),
            ),
          characterCount = 1234,
        )
        .toGoalSourceCandidate()

    assertEquals("folder-1", candidate.entityId)
    assertEquals(LocalDate(2026, 8, 6), candidate.goal?.dueDate)
    assertEquals(LocalDate(2026, 8, 1), candidate.goal?.createdDate)
    assertEquals(50_000L, candidate.goal?.targetCharacterCount)
  }

  @Test
  fun ancestorGoalDatesDoNotRollOverBeforeKstMidnight() {
    val candidate =
      folderAncestor(
          id = "folder-1",
          goal =
            ancestorGoal(
              dueAt = Instant.parse("2026-08-05T14:59:59Z"),
              createdAt = Instant.parse("2026-07-31T14:59:59Z"),
            ),
          characterCount = 1234,
        )
        .toGoalSourceCandidate()

    assertEquals(LocalDate(2026, 8, 5), candidate.goal?.dueDate)
    assertEquals(LocalDate(2026, 7, 31), candidate.goal?.createdDate)
  }

  @Test
  fun folderAncestorCarriesServerCharacterCount() {
    val candidate =
      folderAncestor(id = "folder-1", goal = ancestorGoal(), characterCount = 8765)
        .toGoalSourceCandidate()

    assertEquals(8765L, candidate.folderCharacterCount)
  }

  @Test
  fun nonFolderAncestorHasNullCharacterCount() {
    val candidate = documentAncestor(id = "doc-1", goal = ancestorGoal()).toGoalSourceCandidate()

    assertNull(candidate.folderCharacterCount)
    assertEquals("goal-ancestor", candidate.goal?.id)
  }

  @Test
  fun ancestorWithoutGoalHasNullGoal() {
    val candidate =
      folderAncestor(id = "folder-1", goal = null, characterCount = 100).toGoalSourceCandidate()

    assertNull(candidate.goal)
    assertEquals(100L, candidate.folderCharacterCount)
  }

  @Test
  fun emptyAncestorsMapToEmptyList() {
    assertEquals(
      emptyList(),
      emptyList<DocumentScreen_Query.Ancestor>().toGoalSourceCandidates(),
    )
  }

  @Test
  fun nonFolderAncestorGoalIsNotAdopted() {
    val ancestors =
      listOf(documentAncestor(id = "doc-1", goal = ancestorGoal(id = "goal-doc")))
        .toGoalSourceCandidates()

    assertNull(
      pickGoalSource(
        entityId = "entity-1",
        ownGoal = null,
        ownCurrent = 400L,
        ancestors = ancestors,
      )
    )
  }

  @Test
  fun nearestAncestorFolderGoalIsAdoptedWithFolderCount() {
    val ancestors =
      listOf(
          folderAncestor(
            id = "folder-root",
            goal = ancestorGoal(id = "goal-root"),
            characterCount = 100,
          ),
          folderAncestor(
            id = "folder-near",
            goal = ancestorGoal(id = "goal-near"),
            characterCount = 900,
          ),
        )
        .toGoalSourceCandidates()

    val source =
      pickGoalSource(
        entityId = "entity-1",
        ownGoal = null,
        ownCurrent = 400L,
        ancestors = ancestors,
      )

    assertEquals("folder-near", source?.entityId)
    assertEquals("goal-near", source?.goal?.id)
    assertEquals(900L, source?.current)
    assertTrue(source?.isFolder == true)
  }

  @Test
  fun ownGoalWinsOverAncestorGoal() {
    val ancestors =
      listOf(
          folderAncestor(
            id = "folder-near",
            goal = ancestorGoal(id = "goal-near"),
            characterCount = 900,
          )
        )
        .toGoalSourceCandidates()

    val source =
      pickGoalSource(
        entityId = "entity-1",
        ownGoal =
          ownGoal(dueAt = null, createdAt = Instant.parse("2026-08-01T00:00:00Z"))
            .entityGoalFields_goal
            .toEntityGoalData(),
        ownCurrent = 400L,
        ancestors = ancestors,
      )

    assertEquals("entity-1", source?.entityId)
    assertEquals("goal-own", source?.goal?.id)
    assertEquals(400L, source?.current)
    assertEquals(false, source?.isFolder)
  }
}
