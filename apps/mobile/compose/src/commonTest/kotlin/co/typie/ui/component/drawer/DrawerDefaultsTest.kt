package co.typie.ui.component.drawer

import androidx.compose.animation.core.AnimationVector1D
import androidx.compose.animation.core.FastOutSlowInEasing
import androidx.compose.animation.core.VectorConverter
import kotlin.test.Test
import kotlin.test.assertEquals

class DrawerDefaultsTest {
  @Test
  fun `drawer animation uses the Material3 duration and easing`() {
    val vectorized = DrawerDefaults.AnimationSpec.vectorize(Float.VectorConverter)
    val initial = AnimationVector1D(0f)
    val target = AnimationVector1D(1f)
    val initialVelocity = AnimationVector1D(0f)

    assertEquals(256_000_000L, vectorized.getDurationNanos(initial, target, initialVelocity))
    assertEquals(
      FastOutSlowInEasing.transform(0.5f),
      vectorized.getValueFromNanos(128_000_000L, initial, target, initialVelocity).value,
      absoluteTolerance = 0.0001f,
    )
  }
}
