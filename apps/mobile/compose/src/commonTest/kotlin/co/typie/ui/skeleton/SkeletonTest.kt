package co.typie.ui.skeleton

import androidx.compose.runtime.mutableStateOf
import androidx.compose.ui.graphics.Color
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertFalse
import kotlin.test.assertTrue

class SkeletonTest {
  @Test
  fun `enabled skeleton fully covers content before animation catches up`() {
    val state =
      SkeletonState(
        enabled = true,
        animatedFraction = mutableStateOf(0f),
        boneColor = mutableStateOf(Color.Transparent),
      )

    assertEquals(1f, state.fraction.value)
  }

  @Test
  fun `disabled skeleton follows animation fraction during fade out`() {
    val state =
      SkeletonState(
        enabled = false,
        animatedFraction = mutableStateOf(0.5f),
        boneColor = mutableStateOf(Color.Transparent),
      )

    assertEquals(0.5f, state.fraction.value)
  }

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
