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

  private fun plan(
    pointerX: Float = 150f,
    pointerY: Float,
    density: Float = 1f,
  ): EdgeAutoScrollPlan =
    computeEdgeAutoScrollPlan(
      pointer = Offset(pointerX * density, pointerY * density),
      insetViewport = Rect(0f, 0f, 300f * density, 600f * density),
      density = density,
    )

  @Test
  fun `computeEdgeAutoScrollPlan at viewport center returns no-op`() {
    val result = plan(pointerY = 300f)
    assertTrue(result.isNoOp)
  }

  @Test
  fun `computeEdgeAutoScrollPlan does not scroll at or beyond zone entry`() {
    assertTrue(plan(pointerY = 30f).isNoOp)
    assertTrue(plan(pointerY = 31f).isNoOp)
  }

  @Test
  fun `computeEdgeAutoScrollPlan follows shared speed curve inside top zone`() {
    val justInside = plan(pointerY = 29.999f)
    val halfway = plan(pointerY = 15f)
    val atEdge = plan(pointerY = 0f)

    assertEquals(-1f, justInside.verticalDirection)
    assertEquals(240f, justInside.verticalSpeedPxPerSec, absoluteTolerance = 0.1f)
    assertEquals(600f, halfway.verticalSpeedPxPerSec)
    assertEquals(960f, atEdge.verticalSpeedPxPerSec)
  }

  @Test
  fun `computeEdgeAutoScrollPlan accelerates outside top edge and caps speed`() {
    val tenOutside = plan(pointerY = -10f)
    val twentyOutside = plan(pointerY = -20f)
    val capEntry = plan(pointerY = -28f)
    val farOutside = plan(pointerY = -100f)

    assertEquals(-1f, tenOutside.verticalDirection)
    assertEquals(1260f, tenOutside.verticalSpeedPxPerSec)
    assertEquals(1560f, twentyOutside.verticalSpeedPxPerSec)
    assertEquals(1800f, capEntry.verticalSpeedPxPerSec)
    assertEquals(1800f, farOutside.verticalSpeedPxPerSec)
  }

  @Test
  fun `computeEdgeAutoScrollPlan applies outside curve symmetrically at bottom edge`() {
    val result = plan(pointerY = 620f)

    assertEquals(1f, result.verticalDirection)
    assertEquals(1560f, result.verticalSpeedPxPerSec)
    assertEquals(0f, result.horizontalDirection)
  }

  @Test
  fun `computeEdgeAutoScrollPlan computes horizontal and vertical axes independently`() {
    val result = plan(pointerX = -20f, pointerY = -10f)

    assertEquals(-1f, result.horizontalDirection)
    assertEquals(1560f, result.horizontalSpeedPxPerSec)
    assertEquals(-1f, result.verticalDirection)
    assertEquals(1260f, result.verticalSpeedPxPerSec)
  }

  @Test
  fun `computeEdgeAutoScrollPlan preserves logical curve across densities`() {
    val densityOne = plan(pointerY = -10f, density = 1f)
    val densityTwo = plan(pointerY = -10f, density = 2f)

    assertEquals(1260f, densityOne.verticalSpeedPxPerSec)
    assertEquals(2520f, densityTwo.verticalSpeedPxPerSec)
  }
}
