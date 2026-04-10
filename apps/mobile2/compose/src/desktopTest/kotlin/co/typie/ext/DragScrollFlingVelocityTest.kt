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
        ancestorConsumedLastSample = true,
        localConsumedLastSample = false,
      ),
    )
  }

  @Test
  fun hands_off_to_ancestor_when_last_drag_sample_was_owned_by_ancestor() {
    assertEquals(
      expected = true,
      actual = shouldHandOffDragScrollFlingToAncestorImmediately(
        ancestorParticipated = true,
        localParticipated = true,
        ancestorConsumedLastSample = true,
        localConsumedLastSample = false,
      ),
    )
  }

  @Test
  fun keeps_local_fling_when_ancestor_only_participated_earlier() {
    assertEquals(
      expected = false,
      actual = shouldHandOffDragScrollFlingToAncestorImmediately(
        ancestorParticipated = true,
        localParticipated = true,
        ancestorConsumedLastSample = false,
        localConsumedLastSample = true,
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

  @Test
  fun scrollable_content_handoff_uses_touch_like_velocity_direction_without_damping() {
    assertEquals(
      expected = -100f,
      actual = resolveDragScrollAncestorHandoffVelocity(
        pointerVelocity = 100f,
        mode = DragScrollFlingMode.ScrollableContent,
      ),
    )
  }

  @Test
  fun direct_bridge_handoff_keeps_pointer_velocity_direction() {
    assertEquals(
      expected = 100f,
      actual = resolveDragScrollAncestorHandoffVelocity(
        pointerVelocity = 100f,
        mode = DragScrollFlingMode.DirectBridge,
      ),
    )
  }
}
