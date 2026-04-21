package co.typie.ui.component.topbar

import androidx.compose.foundation.ScrollState
import kotlin.test.Test
import kotlin.test.assertEquals

class TopBarScrollOffsetTest {

  @Test
  fun scrollStateOffsetReturnsValue() {
    val scrollState = ScrollState(initial = 100)
    val offsetFn = scrollState.topBarScrollOffset()
    assertEquals(100, offsetFn())
  }

  @Test
  fun scrollStateZeroOffset() {
    val scrollState = ScrollState(initial = 0)
    val offsetFn = scrollState.topBarScrollOffset()
    assertEquals(0, offsetFn())
  }
}
