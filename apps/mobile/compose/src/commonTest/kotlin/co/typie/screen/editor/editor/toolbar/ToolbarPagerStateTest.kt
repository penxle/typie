package co.typie.screen.editor.editor.toolbar

import kotlin.test.Test
import kotlin.test.assertEquals

class ToolbarPagerStateTest {
  @Test
  fun metrics_keep_progress_within_internal_scroll_before_page_transition() {
    val metrics = ToolbarPagerMetrics(pageDistance = 300f, scrollRanges = listOf(120, 0, 0))

    assertEquals(0f, metrics.progressFor(60f))
    assertEquals(0f, metrics.progressFor(120f))
    assertEquals(0.5f, metrics.progressFor(270f))
    assertEquals(1f, metrics.progressFor(420f))
  }

  @Test
  fun metrics_stop_once_at_internal_scroll_edge_before_crossing_pages() {
    val metrics = ToolbarPagerMetrics(pageDistance = 300f, scrollRanges = listOf(120, 0))

    val result =
      metrics.applyHardStop(
        currentPosition = 110f,
        proposedPosition = 160f,
        hardStop = null,
        gestureStartPosition = 60f,
        activationEpsilon = 10f,
      )

    assertEquals(120f, result.position)
    assertEquals(ToolbarHardStop(position = 120f, blockedDirection = 1), result.hardStop)
    assertEquals(40f, result.rejectedDelta)
  }

  @Test
  fun metrics_snap_using_resolved_velocity_threshold() {
    val metrics = ToolbarPagerMetrics(pageDistance = 300f, scrollRanges = listOf(120, 0, 0))

    assertEquals(
      120f,
      metrics.snapPosition(
        position = 150f,
        velocity = -1000f,
        hardStop = null,
        swipeVelocityThreshold = 1200f,
      ),
    )
    assertEquals(
      420f,
      metrics.snapPosition(
        position = 150f,
        velocity = -1400f,
        hardStop = null,
        swipeVelocityThreshold = 1200f,
      ),
    )
  }

  @Test
  fun metrics_keep_position_inside_internal_scroll_range() {
    val metrics = ToolbarPagerMetrics(pageDistance = 300f, scrollRanges = listOf(120, 0))

    assertEquals(
      60f,
      metrics.snapPosition(
        position = 60f,
        velocity = 0f,
        hardStop = null,
        swipeVelocityThreshold = 1200f,
      ),
    )
    assertEquals(true, metrics.decaysFlingWithinInternalScroll(position = 60f, velocity = 100f))
  }
}
