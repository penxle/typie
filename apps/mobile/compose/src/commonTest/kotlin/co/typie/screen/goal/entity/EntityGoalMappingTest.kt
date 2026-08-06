package co.typie.screen.goal.entity

import co.typie.domain.goal.CharacterCountPoint
import co.typie.graphql.EntityGoalScreen_Query
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.time.Instant
import kotlinx.datetime.LocalDate

class EntityGoalMappingTest {
  private fun historyEntry(date: Instant, characterCount: Int) =
    EntityGoalScreen_Query.CharacterCountHistory(
      __typename = "EntityCharacterCountHistory",
      date = date,
      characterCount = characterCount,
    )

  @Test
  fun historyPointsUseKstCalendarDay() {
    val points =
      listOf(
          historyEntry(Instant.parse("2026-08-03T15:00:00Z"), 1200),
          historyEntry(Instant.parse("2026-08-04T15:00:00Z"), 1500),
        )
        .toCharacterCountPoints()

    assertEquals(
      listOf(
        CharacterCountPoint(LocalDate(2026, 8, 4), 1200L),
        CharacterCountPoint(LocalDate(2026, 8, 5), 1500L),
      ),
      points,
    )
  }

  @Test
  fun historyPointsAreSortedAscending() {
    val points =
      listOf(
          historyEntry(Instant.parse("2026-08-04T15:00:00Z"), 1500),
          historyEntry(Instant.parse("2026-08-02T15:00:00Z"), 900),
          historyEntry(Instant.parse("2026-08-03T15:00:00Z"), 1200),
        )
        .toCharacterCountPoints()

    assertEquals(
      listOf(LocalDate(2026, 8, 3), LocalDate(2026, 8, 4), LocalDate(2026, 8, 5)),
      points.map { it.date },
    )
    assertEquals(listOf(900L, 1200L, 1500L), points.map { it.characterCount })
  }

  @Test
  fun emptyHistoryMapsToEmptyList() {
    assertEquals(
      emptyList(),
      emptyList<EntityGoalScreen_Query.CharacterCountHistory>().toCharacterCountPoints(),
    )
  }

  @Test
  fun documentNodeExposesTitleAndCount() {
    val node =
      EntityGoalScreen_Query.Node(
        __typename = "Document",
        onDocument =
          EntityGoalScreen_Query.OnDocument(id = "doc-1", title = "홍길동전", characterCount = 4321),
        onFolder = null,
      )

    assertEquals("홍길동전", node.targetName())
    assertEquals(4321L, node.currentCharacterCount())
  }

  @Test
  fun folderNodeExposesNameAndCount() {
    val node =
      EntityGoalScreen_Query.Node(
        __typename = "Folder",
        onDocument = null,
        onFolder =
          EntityGoalScreen_Query.OnFolder(id = "folder-1", name = "보관함", characterCount = 8765),
      )

    assertEquals("보관함", node.targetName())
    assertEquals(8765L, node.currentCharacterCount())
  }

  @Test
  fun blankDocumentTitleFallsBackToUntitledText() {
    val blank =
      EntityGoalScreen_Query.Node(
        __typename = "Document",
        onDocument =
          EntityGoalScreen_Query.OnDocument(id = "doc-1", title = "", characterCount = 0),
        onFolder = null,
      )
    val whitespace =
      EntityGoalScreen_Query.Node(
        __typename = "Document",
        onDocument =
          EntityGoalScreen_Query.OnDocument(id = "doc-2", title = "   ", characterCount = 0),
        onFolder = null,
      )

    assertEquals("(제목 없음)", blank.targetName())
    assertEquals("(제목 없음)", whitespace.targetName())
  }

  @Test
  fun blankFolderNameFallsBackToUnnamedText() {
    val node =
      EntityGoalScreen_Query.Node(
        __typename = "Folder",
        onDocument = null,
        onFolder = EntityGoalScreen_Query.OnFolder(id = "folder-1", name = "", characterCount = 0),
      )

    assertEquals("(이름 없음)", node.targetName())
  }

  @Test
  fun unknownNodeFallsBackToUntitledTextAndZeroCount() {
    val node =
      EntityGoalScreen_Query.Node(__typename = "Unknown", onDocument = null, onFolder = null)

    assertEquals("(제목 없음)", node.targetName())
    assertEquals(0L, node.currentCharacterCount())
  }
}
