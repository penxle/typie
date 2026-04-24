package co.typie.ext

import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.geometry.Rect
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertTrue

class EdgeAutoScrollTest {
  @Test
  fun `insetEdgeAutoScrollViewportRect applies top and bottom viewport insets`() {
    assertEquals(
      Rect(left = 24f, top = 331f, right = 846f, bottom = 3260f),
      insetEdgeAutoScrollViewportRect(
        viewport = Rect(left = 24f, top = 283f, right = 846f, bottom = 3404f),
        topInsetPx = 48f,
        bottomInsetPx = 144f,
      ),
    )
  }

  @Test
  fun `insetEdgeAutoScrollViewportRect with zero insets returns identical rect`() {
    val rect = Rect(left = 0f, top = 0f, right = 100f, bottom = 200f)
    assertEquals(rect, insetEdgeAutoScrollViewportRect(rect, topInsetPx = 0f, bottomInsetPx = 0f))
  }

  @Test
  fun `insetEdgeAutoScrollViewportRect clamps top inset exceeding viewport height`() {
    val result =
      insetEdgeAutoScrollViewportRect(
        viewport = Rect(left = 0f, top = 0f, right = 100f, bottom = 200f),
        topInsetPx = 500f,
        bottomInsetPx = 0f,
      )
    assertEquals(200f, result.top)
    assertEquals(200f, result.bottom)
  }

  @Test
  fun `insetEdgeAutoScrollViewportRect clamps bottom inset exceeding viewport height`() {
    val result =
      insetEdgeAutoScrollViewportRect(
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

  private fun plan(pointer: Offset): EdgeAutoScrollPlan =
    computeEdgeAutoScrollPlan(
      pointer = pointer,
      insetViewport = viewport,
      edgeThresholdPx = thresholdPx,
      minSpeedPxPerSec = minSpeedPxPerSec,
      maxSpeedPxPerSec = maxSpeedPxPerSec,
    )

  @Test
  fun `computeEdgeAutoScrollPlan at viewport center returns no-op`() {
    val result = plan(Offset(x = 250f, y = 500f))
    assertTrue(result.isNoOp)
  }

  @Test
  fun `computeEdgeAutoScrollPlan in top edge zone scrolls up`() {
    val result = plan(Offset(x = 250f, y = 25f))
    assertEquals(-1f, result.verticalDirection)
    assertEquals(0f, result.horizontalDirection)
  }

  @Test
  fun `computeEdgeAutoScrollPlan in bottom edge zone scrolls down`() {
    val result = plan(Offset(x = 250f, y = 975f))
    assertEquals(1f, result.verticalDirection)
    assertEquals(0f, result.horizontalDirection)
  }

  @Test
  fun `computeEdgeAutoScrollPlan in left edge zone scrolls left`() {
    val result = plan(Offset(x = 25f, y = 500f))
    assertEquals(0f, result.verticalDirection)
    assertEquals(-1f, result.horizontalDirection)
  }

  @Test
  fun `computeEdgeAutoScrollPlan in right edge zone scrolls right`() {
    val result = plan(Offset(x = 475f, y = 500f))
    assertEquals(0f, result.verticalDirection)
    assertEquals(1f, result.horizontalDirection)
  }

  @Test
  fun `computeEdgeAutoScrollPlan in top-left corner scrolls both axes`() {
    val result = plan(Offset(x = 25f, y = 25f))
    assertEquals(-1f, result.verticalDirection)
    assertEquals(-1f, result.horizontalDirection)
  }

  @Test
  fun `computeEdgeAutoScrollPlan at exact top edge returns maxSpeed`() {
    val result = plan(Offset(x = 250f, y = 0f))
    assertEquals(maxSpeedPxPerSec, result.verticalSpeedPxPerSec)
  }

  @Test
  fun `computeEdgeAutoScrollPlan at threshold boundary is outside edge zone`() {
    val result = plan(Offset(x = 250f, y = thresholdPx))
    assertTrue(result.isNoOp)
  }

  @Test
  fun `computeEdgeAutoScrollPlan speed scales linearly between min and max`() {
    val result = plan(Offset(x = 250f, y = thresholdPx / 2f))
    val expected = minSpeedPxPerSec + 0.5f * (maxSpeedPxPerSec - minSpeedPxPerSec)
    assertEquals(expected, result.verticalSpeedPxPerSec, absoluteTolerance = 1f)
  }

  @Test
  fun `computeEdgeAutoScrollPlan above viewport treats as at edge with maxSpeed`() {
    val result = plan(Offset(x = 250f, y = -50f))
    assertEquals(-1f, result.verticalDirection)
    assertEquals(maxSpeedPxPerSec, result.verticalSpeedPxPerSec)
  }

  @Test
  fun `computeEdgeAutoScrollPlan below viewport treats as at edge with maxSpeed`() {
    val result = plan(Offset(x = 250f, y = 1100f))
    assertEquals(1f, result.verticalDirection)
    assertEquals(maxSpeedPxPerSec, result.verticalSpeedPxPerSec)
  }
}
