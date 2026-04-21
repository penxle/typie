package co.typie.ext

import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.geometry.Rect
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertTrue

class AutoScrollTest {
  @Test
  fun `insetViewportRect applies top and bottom viewport insets`() {
    assertEquals(
      Rect(left = 24f, top = 331f, right = 846f, bottom = 3260f),
      insetViewportRect(
        viewport = Rect(left = 24f, top = 283f, right = 846f, bottom = 3404f),
        topInsetPx = 48f,
        bottomInsetPx = 144f,
      ),
    )
  }

  @Test
  fun `insetViewportRect with zero insets returns identical rect`() {
    val rect = Rect(left = 0f, top = 0f, right = 100f, bottom = 200f)
    assertEquals(rect, insetViewportRect(rect, topInsetPx = 0f, bottomInsetPx = 0f))
  }

  @Test
  fun `insetViewportRect clamps top inset exceeding viewport height`() {
    val result =
      insetViewportRect(
        viewport = Rect(left = 0f, top = 0f, right = 100f, bottom = 200f),
        topInsetPx = 500f,
        bottomInsetPx = 0f,
      )
    assertEquals(200f, result.top)
    assertEquals(200f, result.bottom)
  }

  @Test
  fun `insetViewportRect clamps bottom inset exceeding viewport height`() {
    val result =
      insetViewportRect(
        viewport = Rect(left = 0f, top = 0f, right = 100f, bottom = 200f),
        topInsetPx = 0f,
        bottomInsetPx = 500f,
      )
    assertEquals(0f, result.top)
    assertEquals(0f, result.bottom)
  }

  private val viewport = Rect(left = 0f, top = 0f, right = 500f, bottom = 1000f)
  private val thresholdPx = 50f
  private val minSpeedPxPerSec = 100f
  private val maxSpeedPxPerSec = 500f

  private fun plan(pointer: Offset): ScrollPlan =
    computeScrollPlan(
      pointer = pointer,
      insetViewport = viewport,
      edgeThresholdPx = thresholdPx,
      minSpeedPxPerSec = minSpeedPxPerSec,
      maxSpeedPxPerSec = maxSpeedPxPerSec,
    )

  @Test
  fun `computeScrollPlan at viewport center returns no-op`() {
    val result = plan(Offset(x = 250f, y = 500f))
    assertTrue(result.isNoOp)
  }

  @Test
  fun `computeScrollPlan in top edge zone scrolls up`() {
    val result = plan(Offset(x = 250f, y = 25f))
    assertEquals(-1f, result.verticalDirection)
    assertEquals(0f, result.horizontalDirection)
  }

  @Test
  fun `computeScrollPlan in bottom edge zone scrolls down`() {
    val result = plan(Offset(x = 250f, y = 975f))
    assertEquals(1f, result.verticalDirection)
    assertEquals(0f, result.horizontalDirection)
  }

  @Test
  fun `computeScrollPlan in left edge zone scrolls left`() {
    val result = plan(Offset(x = 25f, y = 500f))
    assertEquals(0f, result.verticalDirection)
    assertEquals(-1f, result.horizontalDirection)
  }

  @Test
  fun `computeScrollPlan in right edge zone scrolls right`() {
    val result = plan(Offset(x = 475f, y = 500f))
    assertEquals(0f, result.verticalDirection)
    assertEquals(1f, result.horizontalDirection)
  }

  @Test
  fun `computeScrollPlan in top-left corner scrolls both axes`() {
    val result = plan(Offset(x = 25f, y = 25f))
    assertEquals(-1f, result.verticalDirection)
    assertEquals(-1f, result.horizontalDirection)
  }

  @Test
  fun `computeScrollPlan at exact top edge returns maxSpeed`() {
    val result = plan(Offset(x = 250f, y = 0f))
    assertEquals(maxSpeedPxPerSec, result.verticalSpeedPxPerSec)
  }

  @Test
  fun `computeScrollPlan at threshold boundary is outside edge zone`() {
    val result = plan(Offset(x = 250f, y = thresholdPx))
    assertTrue(result.isNoOp)
  }

  @Test
  fun `computeScrollPlan speed scales linearly between min and max`() {
    val result = plan(Offset(x = 250f, y = thresholdPx / 2f))
    val expected = minSpeedPxPerSec + 0.5f * (maxSpeedPxPerSec - minSpeedPxPerSec)
    assertEquals(expected, result.verticalSpeedPxPerSec, absoluteTolerance = 1f)
  }

  @Test
  fun `computeScrollPlan above viewport treats as at edge with maxSpeed`() {
    val result = plan(Offset(x = 250f, y = -50f))
    assertEquals(-1f, result.verticalDirection)
    assertEquals(maxSpeedPxPerSec, result.verticalSpeedPxPerSec)
  }

  @Test
  fun `computeScrollPlan below viewport treats as at edge with maxSpeed`() {
    val result = plan(Offset(x = 250f, y = 1100f))
    assertEquals(1f, result.verticalDirection)
    assertEquals(maxSpeedPxPerSec, result.verticalSpeedPxPerSec)
  }
}
