package co.typie.domain.goal

import kotlin.test.Test
import kotlin.test.assertEquals
import kotlinx.datetime.LocalDate

class GoalMetricContentTest {
  private fun source(
    current: Long,
    target: Long,
    isFolder: Boolean = false,
    entityId: String = "entity-1",
  ) =
    GoalSource(
      goal =
        EntityGoalData(
          id = "goal-1",
          targetCharacterCount = target,
          dueDate = null,
          createdDate = LocalDate(2026, 8, 1),
        ),
      current = current,
      isFolder = isFolder,
      entityId = entityId,
    )

  @Test
  fun labelIsGoalForOwnSource() {
    assertEquals("목표", goalMetricLabel(source(current = 100, target = 1000)))
  }

  @Test
  fun labelIsFolderGoalForInheritedSource() {
    assertEquals(
      "폴더 목표",
      goalMetricLabel(source(current = 100, target = 1000, isFolder = true)),
    )
  }

  @Test
  fun labelIsGoalWhenSourceIsAbsent() {
    assertEquals("목표", goalMetricLabel(null))
  }

  @Test
  fun valueIsNotSetWhenSourceIsAbsent() {
    assertEquals("설정 안 함", goalMetricValue(null))
  }

  @Test
  fun valueShowsCommaSeparatedCountsAndPercent() {
    assertEquals(
      "12,400 / 50,000자 (24%)",
      goalMetricValue(source(current = 12_400, target = 50_000)),
    )
  }

  @Test
  fun percentTruncatesTowardZero() {
    assertEquals("999 / 1,000자 (99%)", goalMetricValue(source(current = 999, target = 1000)))
  }

  @Test
  fun percentExceedsHundredWhenOverTarget() {
    assertEquals(
      "60,000 / 50,000자 (120%)",
      goalMetricValue(source(current = 60_000, target = 50_000)),
    )
  }

  @Test
  fun zeroProgressIsZeroPercent() {
    assertEquals("0 / 50,000자 (0%)", goalMetricValue(source(current = 0, target = 50_000)))
  }
}
