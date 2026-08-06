package co.typie.domain.goal

import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.unit.Density
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertNull
import kotlinx.datetime.LocalDate

class UserGoalDotsTest {
  @Test
  fun metricsSeparateWrapSpacingFromPlacementSpacing() {
    val metrics = dotMetrics(Density(2.75f))

    assertEquals(28, metrics.dotPx)
    assertEquals(36, metrics.stridePx)
    assertEquals(9, metrics.wrapGapPx)
  }

  @Test
  fun metricsAgreeOnDensitiesWithWholePixelSpacing() {
    val metrics = dotMetrics(Density(2f))

    assertEquals(20, metrics.dotPx)
    assertEquals(26, metrics.stridePx)
    assertEquals(6, metrics.wrapGapPx)
  }

  @Test
  fun perRowUsesCeiledWrapSpacing() {
    val metrics = dotMetrics(Density(2.75f))

    assertEquals(29, dotsPerRow(metrics, 1080))
    assertEquals(27, dotsPerRow(metrics, 992))
    assertEquals(1, dotsPerRow(metrics, 10))
    assertEquals(0, dotsPerRow(metrics, 0))
  }

  @Test
  fun perRowOnDensityWithoutSpacingDivergence() {
    val metrics = dotMetrics(Density(1f))

    assertEquals(25, dotsPerRow(metrics, 328))
    assertEquals(28, dotsPerRow(metrics, 361))
  }

  @Test
  fun indexAtMapsTapsOntoDotsAtFractionalDensity() {
    val metrics = dotMetrics(Density(2.75f))
    val perRow = dotsPerRow(metrics, 1080)

    assertEquals(0, dotIndexAt(Offset(0f, 0f), perRow, metrics.stridePx, 112))
    assertEquals(0, dotIndexAt(Offset(27f, 27f), perRow, metrics.stridePx, 112))
    assertEquals(1, dotIndexAt(Offset(36f, 0f), perRow, metrics.stridePx, 112))
    assertEquals(28, dotIndexAt(Offset(1018f, 5f), perRow, metrics.stridePx, 112))
    assertEquals(29, dotIndexAt(Offset(5f, 40f), perRow, metrics.stridePx, 112))
    assertEquals(58, dotIndexAt(Offset(5f, 80f), perRow, metrics.stridePx, 112))
  }

  @Test
  fun indexAtRejectsTapsOutsideTheGrid() {
    val metrics = dotMetrics(Density(2.75f))
    val perRow = dotsPerRow(metrics, 1080)

    assertNull(dotIndexAt(Offset(1044f, 5f), perRow, metrics.stridePx, 112))
    assertNull(dotIndexAt(Offset(-1f, 5f), perRow, metrics.stridePx, 112))
    assertNull(dotIndexAt(Offset(5f, -1f), perRow, metrics.stridePx, 112))
    assertNull(dotIndexAt(Offset(905f, 113f), perRow, metrics.stridePx, 112))
    assertNull(dotIndexAt(Offset(5f, 5f), perRow = 0, stridePx = metrics.stridePx, count = 112))
  }

  @Test
  fun dotStateTreatsMissingGoalBeforeJudgment() {
    val date = LocalDate(2026, 8, 5)

    assertEquals(DotState.NoGoal, dotState(DotDay(date, null, null)))
    assertEquals(DotState.NoGoal, dotState(DotDay(date, null, true)))
    assertEquals(DotState.Achieved, dotState(DotDay(date, 1200, true)))
    assertEquals(DotState.Partial, dotState(DotDay(date, 300, false)))
    assertEquals(DotState.Missed, dotState(DotDay(date, 0, false)))
  }
}
