package co.typie.domain.goal

import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertNull
import kotlinx.datetime.LocalDate

class EntityGoalTrendChartTest {
  private val today = LocalDate(2026, 8, 6)

  private fun point(day: Int, characterCount: Long) =
    CharacterCountPoint(LocalDate(2026, 8, day), characterCount)

  private fun goal(target: Long = 1000, dueDay: Int? = null, createdDay: Int = 1) =
    EntityGoalData(
      id = "goal",
      targetCharacterCount = target,
      dueDate = dueDay?.let { LocalDate(2026, 8, it) },
      createdDate = LocalDate(2026, 8, createdDay),
    )

  @Test
  fun emptyHistoryAnchorsBothEndsToToday() {
    val scale = goalTrendScale(history = emptyList(), current = 0, goal = null, today = today)

    assertEquals(today, scale.first)
    assertEquals(today, scale.last)
    assertEquals(1, scale.spanDays)
    assertEquals(1.05, scale.yMax, 1e-9)
  }

  @Test
  fun firstComesFromLeadingHistoryPoint() {
    val history = listOf(point(2, 100), point(4, 300))
    val scale = goalTrendScale(history, current = 300, goal = null, today = today)

    assertEquals(LocalDate(2026, 8, 2), scale.first)
    assertEquals(today, scale.last)
    assertEquals(4, scale.spanDays)
  }

  @Test
  fun dueDateExtendsLastOnlyWhenItIsAfterToday() {
    val history = listOf(point(2, 100))

    val future = goalTrendScale(history, current = 100, goal = goal(dueDay = 20), today = today)
    assertEquals(LocalDate(2026, 8, 20), future.last)
    assertEquals(18, future.spanDays)

    val past = goalTrendScale(history, current = 100, goal = goal(dueDay = 3), today = today)
    assertEquals(today, past.last)
    assertEquals(4, past.spanDays)
  }

  @Test
  fun spanDaysNeverDropsBelowOne() {
    val history = listOf(point(6, 100))
    val scale = goalTrendScale(history, current = 100, goal = null, today = today)

    assertEquals(today, scale.first)
    assertEquals(today, scale.last)
    assertEquals(1, scale.spanDays)
  }

  @Test
  fun yMaxTakesLargestOfFloorTargetCurrentAndHistory() {
    val history = listOf(point(2, 100), point(4, 3000))

    assertEquals(
      1050.0,
      goalTrendScale(history = emptyList(), current = 50, goal = goal(target = 1000), today = today)
        .yMax,
      1e-9,
    )
    assertEquals(
      2100.0,
      goalTrendScale(
          listOf(point(2, 100)),
          current = 2000,
          goal = goal(target = 1000),
          today = today,
        )
        .yMax,
      1e-9,
    )
    assertEquals(
      3150.0,
      goalTrendScale(history, current = 50, goal = goal(target = 1000), today = today).yMax,
      1e-9,
    )
    assertEquals(
      1.05,
      goalTrendScale(listOf(point(2, 0)), current = 0, goal = null, today = today).yMax,
      1e-9,
    )
  }

  @Test
  fun xMapsDateSpanOntoPlotWidth() {
    val scale =
      GoalTrendScale(
        first = LocalDate(2026, 8, 1),
        last = LocalDate(2026, 8, 11),
        spanDays = 10,
        yMax = 200.0,
      )

    assertEquals(8f, scale.x(LocalDate(2026, 8, 1), 8f, 100f), 1e-4f)
    assertEquals(58f, scale.x(LocalDate(2026, 8, 6), 8f, 100f), 1e-4f)
    assertEquals(108f, scale.x(LocalDate(2026, 8, 11), 8f, 100f), 1e-4f)
  }

  @Test
  fun yMapsValueOntoInvertedPlotHeight() {
    val scale =
      GoalTrendScale(
        first = LocalDate(2026, 8, 1),
        last = LocalDate(2026, 8, 11),
        spanDays = 10,
        yMax = 200.0,
      )

    assertEquals(180f, scale.y(0, 8f, 172f), 1e-4f)
    assertEquals(94f, scale.y(100, 8f, 172f), 1e-4f)
    assertEquals(8f, scale.y(200, 8f, 172f), 1e-4f)
  }

  @Test
  fun paceIsAbsentWithoutGoalOrDueDate() {
    val history = listOf(point(2, 100))

    assertNull(goalTrendPace(history, goal = null))
    assertNull(goalTrendPace(history, goal = goal(dueDay = null)))
    assertNull(goalTrendPace(history = emptyList(), goal = goal(dueDay = 20)))
  }

  @Test
  fun paceAnchorsOnLastPointNotAfterCreatedDate() {
    val history = listOf(point(1, 100), point(3, 300), point(5, 500))
    val pace = goalTrendPace(history, goal = goal(dueDay = 20, createdDay = 4))

    assertEquals(GoalTrendPace(date = LocalDate(2026, 8, 4), value = 300), pace)
  }

  @Test
  fun paceKeepsAnchorDateWhenPointSitsOnCreatedDate() {
    val history = listOf(point(1, 100), point(3, 300), point(5, 500))
    val pace = goalTrendPace(history, goal = goal(dueDay = 20, createdDay = 3))

    assertEquals(GoalTrendPace(date = LocalDate(2026, 8, 3), value = 300), pace)
  }

  @Test
  fun paceFallsBackToFirstPointWhenAllPointsFollowCreatedDate() {
    val history = listOf(point(5, 500), point(7, 700))
    val pace = goalTrendPace(history, goal = goal(dueDay = 20, createdDay = 1))

    assertEquals(GoalTrendPace(date = LocalDate(2026, 8, 5), value = 500), pace)
  }

  @Test
  fun monthDayLabelDropsLeadingZeros() {
    assertEquals("8월 6일", goalMonthDayLabel(LocalDate(2026, 8, 6)))
    assertEquals("12월 25일", goalMonthDayLabel(LocalDate(2026, 12, 25)))
    assertEquals("1월 1일", goalMonthDayLabel(LocalDate(2026, 1, 1)))
  }
}
