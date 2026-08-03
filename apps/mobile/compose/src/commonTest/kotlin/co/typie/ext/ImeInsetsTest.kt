package co.typie.ext

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
}
