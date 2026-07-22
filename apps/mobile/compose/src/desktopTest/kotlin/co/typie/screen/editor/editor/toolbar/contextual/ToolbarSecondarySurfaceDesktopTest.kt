package co.typie.screen.editor.editor.toolbar.contextual

import androidx.compose.foundation.ScrollState
import androidx.compose.foundation.horizontalScroll
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.relocation.BringIntoViewRequester
import androidx.compose.foundation.relocation.bringIntoViewRequester
import androidx.compose.runtime.Composable
import androidx.compose.runtime.CompositionLocalProvider
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.MutableIntState
import androidx.compose.runtime.mutableIntStateOf
import androidx.compose.runtime.remember
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.LocalDensity
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.test.ComposeUiTest
import androidx.compose.ui.test.ExperimentalTestApi
import androidx.compose.ui.test.onNodeWithTag
import androidx.compose.ui.test.v2.runComposeUiTest
import androidx.compose.ui.unit.Density
import androidx.compose.ui.unit.dp
import co.typie.screen.editor.editor.toolbar.ToolbarItemGap
import co.typie.screen.editor.editor.toolbar.ToolbarPageEndPadding
import co.typie.screen.editor.editor.toolbar.ToolbarSecondaryContentStartInset
import co.typie.ui.theme.LightAppShadows
import co.typie.ui.theme.LightColors
import co.typie.ui.theme.LocalAppColors
import co.typie.ui.theme.LocalAppShadows
import co.typie.ui.theme.LocalThemeMode
import co.typie.ui.theme.ResolvedThemeMode
import dev.chrisbanes.haze.blur.HazeBlurStyle
import dev.chrisbanes.haze.blur.LocalHazeBlurStyle
import kotlin.math.roundToInt
import kotlin.test.Test
import kotlin.test.assertTrue

@OptIn(ExperimentalTestApi::class)
class ToolbarSecondarySurfaceDesktopTest {
  @Test
  fun leftEdgeRevealKeepsContentPaddingAfterStartOverlay() = runComposeUiTest {
    val contentState = setToolbarContent(targetIndex = 1, initialScroll = LeftTargetInitialScroll)
    val surfaceBounds = onNodeWithTag(SurfaceTag).fetchSemanticsNode().boundsInRoot
    val initialTargetBounds = onNodeWithTag(TargetTag).fetchSemanticsNode().boundsInRoot
    val visibleStart =
      surfaceBounds.left + ToolbarSecondaryContentStartInset.value + ToolbarPageEndPadding.value

    assertTrue(
      initialTargetBounds.left < visibleStart,
      "test setup should place the target under the start overlay",
    )

    requestBringIntoView(contentState.requestRevision)

    val revealedTargetBounds = onNodeWithTag(TargetTag).fetchSemanticsNode().boundsInRoot
    assertTrue(
      revealedTargetBounds.left >= visibleStart - PositionTolerance,
      "revealed target should not remain under the start overlay. " +
        "target=${revealedTargetBounds.left} visibleStart=$visibleStart",
    )
  }

  @Test
  fun rightEdgeRevealKeepsContentPaddingBeforePhysicalEnd() = runComposeUiTest {
    val contentState = setToolbarContent(targetIndex = 4, initialScroll = 0)
    val surfaceBounds = onNodeWithTag(SurfaceTag).fetchSemanticsNode().boundsInRoot
    val visibleEnd = surfaceBounds.right - ToolbarPageEndPadding.value

    assertTrue(contentState.scrollState.value == 0, "test setup should start at the leading edge")

    requestBringIntoView(contentState.requestRevision)

    val revealedTargetBounds = onNodeWithTag(TargetTag).fetchSemanticsNode().boundsInRoot
    assertTrue(
      contentState.scrollState.value > 0,
      "revealing the target should scroll toward the end",
    )
    assertTrue(
      revealedTargetBounds.right <= visibleEnd + PositionTolerance,
      "revealed target should keep content padding before the physical end. " +
        "target=${revealedTargetBounds.right} visibleEnd=$visibleEnd",
    )
  }

  private fun ComposeUiTest.setToolbarContent(
    targetIndex: Int,
    initialScroll: Int,
  ): ToolbarContentState {
    val requestRevision = mutableIntStateOf(0)
    val scrollState = ScrollState(initial = initialScroll)

    setContent {
      ToolbarSecondarySurfaceTestTheme {
        Box(Modifier.width(ToolbarWidth).testTag(SurfaceTag)) {
          ToolbarSecondarySurface(onClose = {}, closeContentDescription = "닫기") {
            val requester = remember { BringIntoViewRequester() }

            LaunchedEffect(requestRevision.intValue) {
              if (requestRevision.intValue > 0) {
                requester.bringIntoView()
              }
            }

            Row(
              modifier =
                Modifier.fillMaxSize()
                  .horizontalScroll(scrollState)
                  .padding(start = ToolbarSecondaryContentStartInset),
              verticalAlignment = Alignment.CenterVertically,
              horizontalArrangement = Arrangement.spacedBy(ToolbarItemGap),
            ) {
              repeat(ItemCount) { index ->
                Box(
                  Modifier.size(width = ItemWidth, height = ItemHeight)
                    .then(
                      if (index == targetIndex) {
                        Modifier.bringIntoViewRequester(requester).testTag(TargetTag)
                      } else {
                        Modifier
                      }
                    )
                )
              }
            }
          }
        }
      }
    }
    waitForIdle()
    return ToolbarContentState(requestRevision = requestRevision, scrollState = scrollState)
  }

  private fun ComposeUiTest.requestBringIntoView(requestRevision: MutableIntState) {
    waitForIdle()
    runOnIdle { requestRevision.intValue += 1 }
    waitForIdle()
  }

  @Composable
  private fun ToolbarSecondarySurfaceTestTheme(content: @Composable () -> Unit) {
    CompositionLocalProvider(
      LocalDensity provides Density(1f),
      LocalAppColors provides LightColors,
      LocalAppShadows provides LightAppShadows,
      LocalThemeMode provides ResolvedThemeMode.Light,
      LocalHazeBlurStyle provides
        HazeBlurStyle(blurRadius = 20.dp, noiseFactor = 0f, colorEffects = listOf()),
      content = content,
    )
  }

  private companion object {
    const val SurfaceTag = "secondary-toolbar-surface"
    const val TargetTag = "secondary-toolbar-target"
    const val ItemCount = 6
    const val PositionTolerance = 0.5f
    val ToolbarWidth = 240.dp
    val ItemWidth = 80.dp
    val ItemHeight = 30.dp
    val LeftTargetInitialScroll =
      (ToolbarSecondaryContentStartInset + ItemWidth + ToolbarItemGap).value.roundToInt()
  }

  private data class ToolbarContentState(
    val requestRevision: MutableIntState,
    val scrollState: ScrollState,
  )
}
