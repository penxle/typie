package co.typie.screen.more.stats

import kotlin.test.Test
import kotlin.test.assertEquals
import kotlinx.datetime.LocalDate

class StatsActivityChartZoomTest {

  @Test
  fun calculateChartPinchZoomState_keepsFocalPointStable() {
    val result =
      calculateChartPinchZoomState(
        pinchStartZoom = 2f,
        pinchScale = 1.5f,
        pinchStartOffset = 120f,
        focalX = 80f,
        viewportWidth = 300f,
      )

    assertEquals(3f, result.zoom)
    assertEquals(220f, result.scrollOffset)
  }

  @Test
  fun calculateChartPinchZoomState_clampsZoomAndOffset() {
    val result =
      calculateChartPinchZoomState(
        pinchStartZoom = 3.5f,
        pinchScale = 2f,
        pinchStartOffset = 900f,
        focalX = 320f,
        viewportWidth = 300f,
      )

    assertEquals(4f, result.zoom)
    assertEquals(900f, result.scrollOffset)
  }

  @Test
  fun generateXAxisLabels_showsDenserLabelsWhenZoomedIn() {
    val days =
      List(10) { index ->
        StatsActivityDay(date = LocalDate(2024, 1, 10 + index), additions = 100, deletions = 0)
      }

    assertEquals(listOf(0, 9), generateXAxisLabels(days, zoom = 1f).map { it.index })
    assertEquals(listOf(0, 7, 9), generateXAxisLabels(days, zoom = 1.8f).map { it.index })
    assertEquals(listOf(0, 3, 6, 9), generateXAxisLabels(days, zoom = 2.8f).map { it.index })
  }
}
