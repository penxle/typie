package co.typie.ui.skeleton

import kotlin.test.Test
import kotlin.test.assertFalse
import kotlin.test.assertTrue

class SkeletonTest {
  @Test
  fun `skeleton animation stays active while enabled`() {
    assertTrue(shouldAnimateSkeleton(enabled = true, fraction = 0f))
    assertTrue(shouldAnimateSkeleton(enabled = true, fraction = 1f))
  }

  @Test
  fun `skeleton animation stays active during fade out`() {
    assertTrue(shouldAnimateSkeleton(enabled = false, fraction = 0.5f))
    assertTrue(shouldAnimateSkeleton(enabled = false, fraction = 0.01f))
  }

  @Test
  fun `skeleton animation stops after fade out settles`() {
    assertFalse(shouldAnimateSkeleton(enabled = false, fraction = 0f))
    assertFalse(shouldAnimateSkeleton(enabled = false, fraction = -0.01f))
    assertFalse(shouldAnimateSkeleton(enabled = false, fraction = Float.NaN))
  }
}
