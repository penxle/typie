package co.typie.ext

import kotlin.test.Test
import kotlin.test.assertEquals

class DragScrollFlingVelocityTest {
  @Test
  fun desktop_drag_scroll_uses_elastic_overscroll_when_enabled_and_unlocked() {
    assertEquals(
      expected = true,
      actual =
        shouldUseElasticOverscrollForDesktopDragScroll(
          enabled = true,
          isLocked = false,
          elasticOverscroll = true,
        ),
    )
  }

  @Test
  fun desktop_drag_scroll_can_disable_elastic_overscroll_for_proxy_surfaces() {
    assertEquals(
      expected = false,
      actual =
        shouldUseElasticOverscrollForDesktopDragScroll(
          enabled = true,
          isLocked = false,
          elasticOverscroll = false,
        ),
    )
  }

  @Test
  fun desktop_drag_scroll_skips_elastic_overscroll_when_disabled_or_locked() {
    assertEquals(
      expected = false,
      actual =
        shouldUseElasticOverscrollForDesktopDragScroll(
          enabled = false,
          isLocked = false,
          elasticOverscroll = true,
        ),
    )
    assertEquals(
      expected = false,
      actual =
        shouldUseElasticOverscrollForDesktopDragScroll(
          enabled = true,
          isLocked = true,
          elasticOverscroll = true,
        ),
    )
  }

  @Test
  fun fling_only_handoff_requires_actual_ancestor_owned_release() {
    assertEquals(
      expected = false,
      actual =
        shouldAllowDragScrollFlingAncestorHandoff(
          ancestorParticipated = false,
          ancestorConsumedLastSample = false,
          localConsumedLastSample = false,
        ),
    )
  }

  @Test
  fun prior_ancestor_drag_keeps_release_owned_by_ancestor_even_below_threshold() {
    assertEquals(
      expected = true,
      actual =
        shouldAllowDragScrollFlingAncestorHandoff(
          ancestorParticipated = true,
          ancestorConsumedLastSample = true,
          localConsumedLastSample = false,
        ),
    )
  }

  @Test
  fun earlier_ancestor_participation_does_not_handoff_when_local_owned_release() {
    assertEquals(
      expected = false,
      actual =
        shouldAllowDragScrollFlingAncestorHandoff(
          ancestorParticipated = true,
          ancestorConsumedLastSample = false,
          localConsumedLastSample = true,
        ),
    )
  }

  @Test
  fun weak_boundary_fling_stays_in_elastic_overscroll() {
    assertEquals(
      expected = DragScrollBoundaryFlingOutcome.ContinueElasticOverscroll,
      actual =
        resolveDragScrollBoundaryFlingOutcome(
          availableVelocity = 1800f,
          boundaryUnconsumedScrollDelta = -12f,
          overscrollEnabled = true,
        ),
    )
  }

  @Test
  fun strong_downward_boundary_fling_hands_off_to_ancestor() {
    assertEquals(
      expected = DragScrollBoundaryFlingOutcome.HandOffToAncestor,
      actual =
        resolveDragScrollBoundaryFlingOutcome(
          availableVelocity = 2400f,
          boundaryUnconsumedScrollDelta = -12f,
          overscrollEnabled = true,
        ),
    )
  }

  @Test
  fun non_dismiss_boundary_fling_never_hands_off_to_ancestor() {
    assertEquals(
      expected = DragScrollBoundaryFlingOutcome.ContinueElasticOverscroll,
      actual =
        resolveDragScrollBoundaryFlingOutcome(
          availableVelocity = -3600f,
          boundaryUnconsumedScrollDelta = 12f,
          overscrollEnabled = true,
        ),
    )
  }

  @Test
  fun boundary_elastic_impulse_uses_boundary_direction() {
    assertEquals(
      expected = -36f,
      actual =
        resolveDragScrollBoundaryElasticOverscrollDelta(
          availableVelocity = 1800f,
          boundaryUnconsumedScrollDelta = -8f,
        ),
    )
  }

  @Test
  fun boundary_elastic_impulse_is_clamped() {
    assertEquals(
      expected = -72f,
      actual =
        resolveDragScrollBoundaryElasticOverscrollDelta(
          availableVelocity = 6000f,
          boundaryUnconsumedScrollDelta = -8f,
        ),
    )
  }

  @Test
  fun cancels_local_decay_once_ancestor_takes_over_at_scroll_boundary() {
    assertEquals(
      expected = true,
      actual =
        shouldCancelDragScrollDecayForAncestorHandoff(
          ancestorConsumedPointerDelta = 6.2f,
          localConsumedPointerDelta = 0f,
          unconsumedScrollDelta = 0f,
        ),
    )
  }

  @Test
  fun keeps_local_decay_running_while_scrollable_still_consumes_velocity() {
    assertEquals(
      expected = false,
      actual =
        shouldCancelDragScrollDecayForAncestorHandoff(
          ancestorConsumedPointerDelta = 1.4f,
          localConsumedPointerDelta = 1.3f,
          unconsumedScrollDelta = 0f,
        ),
    )
  }

  @Test
  fun dispatches_post_fling_when_remaining_velocity_exists_without_prior_ancestor_drag() {
    assertEquals(
      expected = true,
      actual =
        shouldDispatchDragScrollPostFlingToAncestor(
          ancestorParticipated = false,
          ancestorConsumedDuringFling = false,
          availableVelocity = 120f,
        ),
    )
  }

  @Test
  fun skips_post_fling_when_no_remaining_velocity_and_ancestor_never_participated() {
    assertEquals(
      expected = false,
      actual =
        shouldDispatchDragScrollPostFlingToAncestor(
          ancestorParticipated = false,
          ancestorConsumedDuringFling = false,
          availableVelocity = 0f,
        ),
    )
  }

  @Test
  fun dispatches_zero_post_fling_when_ancestor_started_consuming_during_decay() {
    assertEquals(
      expected = true,
      actual =
        shouldDispatchDragScrollPostFlingToAncestor(
          ancestorParticipated = false,
          ancestorConsumedDuringFling = true,
          availableVelocity = 0f,
        ),
    )
  }

  @Test
  fun hands_off_to_ancestor_immediately_when_body_drag_only_moved_ancestor() {
    assertEquals(
      expected = true,
      actual =
        shouldHandOffDragScrollFlingToAncestorImmediately(
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
      actual =
        shouldHandOffDragScrollFlingToAncestorImmediately(
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
      actual =
        shouldHandOffDragScrollFlingToAncestorImmediately(
          ancestorParticipated = true,
          localParticipated = true,
          ancestorConsumedLastSample = false,
          localConsumedLastSample = true,
        ),
    )
  }

  @Test
  fun scrollable_content_handoff_uses_touch_like_velocity_direction_without_damping() {
    assertEquals(
      expected = -100f,
      actual =
        resolveDragScrollAncestorHandoffVelocity(
          pointerVelocity = 100f,
          mode = DragScrollFlingMode.ScrollableContent,
        ),
    )
  }

  @Test
  fun direct_bridge_handoff_keeps_pointer_velocity_direction() {
    assertEquals(
      expected = 100f,
      actual =
        resolveDragScrollAncestorHandoffVelocity(
          pointerVelocity = 100f,
          mode = DragScrollFlingMode.DirectBridge,
        ),
    )
  }
}
