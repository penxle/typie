package co.typie.ext

import androidx.compose.foundation.layout.WindowInsets
import androidx.compose.ui.unit.Density
import androidx.compose.ui.unit.LayoutDirection
import androidx.compose.ui.unit.dp
import kotlin.test.Test
import kotlin.test.assertEquals

class ImeInsetsTest {
  @Test
  fun trustedInsetBoundsRefocusOvershootToSettledInset() {
    assertEquals(350.dp, trustedImeBottomInset(rawImeBottom = 806.dp, settledImeBottom = 350.dp))
  }

  @Test
  fun trustedInsetPreservesImeAnimationBeforeItReachesTarget() {
    assertEquals(120.dp, trustedImeBottomInset(rawImeBottom = 120.dp, settledImeBottom = 350.dp))
  }

  @Test
  fun trustedWindowInsetsReadsLatestRawAndSettledValues() {
    var rawBottom = 806
    var settledBottom: Int? = 350
    val insets =
      trustedImeInsets(
        rawImeInsets =
          object : WindowInsets {
            override fun getLeft(density: Density, layoutDirection: LayoutDirection) = 0

            override fun getTop(density: Density) = 0

            override fun getRight(density: Density, layoutDirection: LayoutDirection) = 0

            override fun getBottom(density: Density) = rawBottom
          },
        settledBottom = { settledBottom },
      )
    val density = Density(1f)

    assertEquals(350, insets.getBottom(density))

    rawBottom = 120
    assertEquals(120, insets.getBottom(density))

    settledBottom = null
    rawBottom = 280
    assertEquals(280, insets.getBottom(density))
  }

  @Test
  fun trustedWindowInsetsUsesPresentationBottomInBothDirections() {
    var rawBottom = 120
    var presentationBottom: Int? = 160
    val insets =
      trustedImeInsets(
        rawImeInsets =
          object : WindowInsets {
            override fun getLeft(density: Density, layoutDirection: LayoutDirection) = 0

            override fun getTop(density: Density) = 0

            override fun getRight(density: Density, layoutDirection: LayoutDirection) = 0

            override fun getBottom(density: Density) = rawBottom
          },
        settledBottom = { null },
        presentationBottom = { presentationBottom },
      )
    val density = Density(1f)

    assertEquals(160, insets.getBottom(density))

    rawBottom = 200
    presentationBottom = 80
    assertEquals(80, insets.getBottom(density))

    presentationBottom = 0
    assertEquals(0, insets.getBottom(density))
  }
}
