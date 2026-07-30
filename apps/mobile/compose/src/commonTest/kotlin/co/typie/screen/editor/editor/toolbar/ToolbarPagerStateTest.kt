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
  fun outer_edge_drag_accumulates_resisted_offset_and_unwinds_before_pager_delta() {
    val dragged =
      ToolbarOuterEdgeDrag()
        .applyRejectedPositionDelta(rejectedDelta = 40f, resistance = 0.5f, limit = 30f)

    val partial = dragged.consumeInwardScrollDelta(delta = 20f, resistance = 0.5f)
    val escaped = partial.drag.consumeInwardScrollDelta(delta = 30f, resistance = 0.5f)

    assertEquals(-20f, dragged.offset)
    assertEquals(-10f, partial.drag.offset)
    assertEquals(0f, partial.remainingDelta)
    assertEquals(0f, escaped.drag.offset)
    assertEquals(10f, escaped.remainingDelta)
  }

  @Test
  fun outer_edge_drag_mirrors_direction_and_caps_visual_offset() {
    val dragged =
      ToolbarOuterEdgeDrag()
        .applyRejectedPositionDelta(rejectedDelta = -100f, resistance = 0.5f, limit = 30f)

    val partial = dragged.consumeInwardScrollDelta(delta = -20f, resistance = 0.5f)

    assertEquals(30f, dragged.offset)
    assertEquals(20f, partial.drag.offset)
    assertEquals(0f, partial.remainingDelta)
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
