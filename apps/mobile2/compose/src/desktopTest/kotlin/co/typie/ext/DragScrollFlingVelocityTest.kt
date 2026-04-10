package co.typie.ext

import kotlin.test.Test
import kotlin.test.assertEquals

class DragScrollFlingVelocityTest {
  @Test
  fun hands_off_to_ancestor_immediately_when_body_drag_only_moved_ancestor() {
    assertEquals(
      expected = true,
      actual = shouldHandOffDragScrollFlingToAncestorImmediately(
        ancestorParticipated = true,
        localParticipated = false,
      ),
    )
  }

  @Test
  fun keeps_local_fling_when_scrollable_consumed_drag() {
    assertEquals(
      expected = false,
      actual = shouldHandOffDragScrollFlingToAncestorImmediately(
        ancestorParticipated = true,
        localParticipated = true,
      ),
    )
  }

  @Test
  fun scrollable_content_flips_pointer_velocity_into_scroll_direction() {
    assertEquals(
      expected = -72f,
      actual = resolveDragScrollFlingVelocity(
        pointerVelocity = 100f,
        mode = DragScrollFlingMode.ScrollableContent,
      ),
    )
  }

  @Test
  fun direct_bridge_keeps_pointer_velocity_direction() {
    assertEquals(
      expected = 72f,
      actual = resolveDragScrollFlingVelocity(
        pointerVelocity = 100f,
        mode = DragScrollFlingMode.DirectBridge,
      ),
    )
  }
}
