package co.typie.ext

import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.WindowInsets
import androidx.compose.foundation.layout.size
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableIntStateOf
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.test.ExperimentalTestApi
import androidx.compose.ui.test.onNodeWithTag
import androidx.compose.ui.test.runComposeUiTest
import androidx.compose.ui.unit.Density
import androidx.compose.ui.unit.LayoutDirection
import androidx.compose.ui.unit.dp
import kotlin.test.Test
import kotlin.test.assertEquals

@OptIn(ExperimentalTestApi::class)
class WindowInsetsDesktopTest {
  @Test
  fun navigationBarsOrImePaddingReadsTheLatestInsetDuringLayout() = runComposeUiTest {
    var bottomInsetPx by mutableIntStateOf(0)
    var compositionCount = 0
    val imeInsets =
      object : WindowInsets {
        override fun getLeft(density: Density, layoutDirection: LayoutDirection) = 30

        override fun getTop(density: Density) = 0

        override fun getRight(density: Density, layoutDirection: LayoutDirection) = 0

        override fun getBottom(density: Density) = bottomInsetPx
      }

    setContent {
      compositionCount += 1
      Box(
        Modifier.size(100.dp)
          .navigationBarsOrImePadding(navigationBars = WindowInsets(0), ime = imeInsets)
          .testTag(ContainerTag)
      ) {
        Box(Modifier.align(Alignment.BottomCenter).size(10.dp).testTag(ContentTag))
      }
    }

    val containerBounds = onNodeWithTag(ContainerTag).fetchSemanticsNode().boundsInRoot
    val initialBounds = onNodeWithTag(ContentTag).fetchSemanticsNode().boundsInRoot
    runOnIdle { bottomInsetPx = 24 }
    waitForIdle()

    val updatedBounds = onNodeWithTag(ContentTag).fetchSemanticsNode().boundsInRoot
    assertEquals(24f, initialBounds.bottom - updatedBounds.bottom, absoluteTolerance = 0.5f)
    assertEquals(containerBounds.center.x, initialBounds.center.x, absoluteTolerance = 0.5f)
    assertEquals(containerBounds.center.x, updatedBounds.center.x, absoluteTolerance = 0.5f)
    assertEquals(1, compositionCount)
  }

  private companion object {
    const val ContainerTag = "window-insets-container"
    const val ContentTag = "window-insets-content"
  }
}
