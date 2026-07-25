package co.typie.editor

import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.size
import androidx.compose.runtime.CompositionLocalProvider
import androidx.compose.ui.Modifier
import androidx.compose.ui.geometry.Size
import androidx.compose.ui.platform.LocalDensity
import androidx.compose.ui.semantics.LiveRegionMode
import androidx.compose.ui.semantics.SemanticsProperties
import androidx.compose.ui.test.ExperimentalTestApi
import androidx.compose.ui.test.SemanticsMatcher
import androidx.compose.ui.test.assert
import androidx.compose.ui.test.onNodeWithTag
import androidx.compose.ui.test.v2.runComposeUiTest
import androidx.compose.ui.unit.Density
import androidx.compose.ui.unit.dp
import co.typie.editor.scroll.EditorVisibleArea
import co.typie.ui.theme.LocalThemeMode
import co.typie.ui.theme.ResolvedThemeMode
import dev.chrisbanes.haze.HazeState
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertTrue
import kotlinx.coroutines.flow.MutableSharedFlow

@OptIn(ExperimentalTestApi::class)
class EditorTapHintDesktopTest {
  @Test
  fun hintFadesInStaysVisibleAndFadesOutWithPreviewTimings() = runComposeUiTest {
    mainClock.autoAdvance = false
    val events = MutableSharedFlow<Unit>(extraBufferCapacity = 1)
    setHintContent(events)

    onNodeWithTag(EditorTapHintTestTag).assertDoesNotExist()
    runOnIdle { assertTrue(events.tryEmit(Unit)) }

    mainClock.advanceTimeBy(EditorTapHintFadeMillis / 2L)
    onNodeWithTag(EditorTapHintTestTag)
      .assertExists()
      .assert(SemanticsMatcher.expectValue(SemanticsProperties.LiveRegion, LiveRegionMode.Polite))

    mainClock.advanceTimeBy(EditorTapHintFadeMillis / 2L + EditorTapHintVisibleMillis)
    onNodeWithTag(EditorTapHintTestTag).assertExists()

    mainClock.advanceTimeBy(EditorTapHintFadeMillis + 32L)
    onNodeWithTag(EditorTapHintTestTag).assertDoesNotExist()
  }

  @Test
  fun repeatedEventExtendsDwellWithoutDisappearingAndRestarting() = runComposeUiTest {
    mainClock.autoAdvance = false
    val events = MutableSharedFlow<Unit>(extraBufferCapacity = 1)
    setHintContent(events)

    runOnIdle { assertTrue(events.tryEmit(Unit)) }
    mainClock.advanceTimeBy(EditorTapHintFadeMillis + 800L)
    runOnIdle { assertTrue(events.tryEmit(Unit)) }

    mainClock.advanceTimeBy(400L)
    onNodeWithTag(EditorTapHintTestTag).assertExists()

    mainClock.advanceTimeBy(EditorTapHintVisibleMillis + EditorTapHintFadeMillis + 1L)
    onNodeWithTag(EditorTapHintTestTag).assertDoesNotExist()
  }

  @Test
  fun readingHintIsCenteredInsideEditorVisibleAreaNotTheFullViewport() = runComposeUiTest {
    mainClock.autoAdvance = false
    val events = MutableSharedFlow<Unit>(extraBufferCapacity = 1)
    setHintContent(
      events = events,
      visibleArea =
        EditorVisibleArea(
          viewport = Size(width = 400f, height = 700f),
          topInset = 100f,
          bottomOcclusionInset = 200f,
        ),
    )

    runOnIdle { assertTrue(events.tryEmit(Unit)) }
    mainClock.advanceTimeBy(EditorTapHintFadeMillis.toLong())

    val centerY = onNodeWithTag(EditorTapHintTestTag).fetchSemanticsNode().boundsInRoot.center.y
    assertEquals(300f, centerY, absoluteTolerance = 0.5f)
  }

  private fun androidx.compose.ui.test.ComposeUiTest.setHintContent(
    events: MutableSharedFlow<Unit>,
    visibleArea: EditorVisibleArea = EditorVisibleArea(viewport = Size(width = 400f, height = 700f)),
  ) {
    setContent {
      CompositionLocalProvider(
        LocalDensity provides Density(1f),
        LocalThemeMode provides ResolvedThemeMode.Light,
      ) {
        Box(Modifier.size(width = 400.dp, height = 700.dp)) {
          EditorTapHintOverlay(
            events = events,
            text = "편집하려면 더블 탭",
            hazeState = HazeState(),
            visibleArea = visibleArea,
          )
        }
      }
    }
  }
}
