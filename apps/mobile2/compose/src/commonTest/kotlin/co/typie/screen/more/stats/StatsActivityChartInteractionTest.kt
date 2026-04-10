package co.typie.screen.more.stats

import kotlin.test.Test
import kotlin.test.assertEquals

class StatsActivityChartInteractionTest {

  @Test
  fun barIndexAtContentPosition_usesContentCoordinatesAfterZoom() {
    val result = barIndexAtContentPosition(
      localContentX = 155f,
      barWidthPx = 10f,
      itemCount = 30,
    )

    assertEquals(15, result)
  }

  @Test
  fun viewportFocalXFromContentPosition_convertsScrolledContentBackToViewport() {
    val result = viewportFocalXFromContentPosition(
      localContentX = 155f,
      scrollOffset = 120f,
      viewportWidthPx = 300f,
    )

    assertEquals(35f, result)
  }

  @Test
  fun calculateChartBarHeights_scalesAdditionsAndDeletionsAgainstAnimatedMaxValue() {
    val result = calculateChartBarHeights(
      additions = 300,
      deletions = 100,
      maxValue = 400f,
      chartHeightPx = 100f,
    )

    assertEquals(75f, result.additionsHeightPx)
    assertEquals(25f, result.deletionsHeightPx)
  }
}
