package co.typie.screen.editor.editor.toolbar

import androidx.compose.foundation.ScrollState
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.rememberScrollState
import androidx.compose.runtime.Composable
import androidx.compose.runtime.CompositionLocalProvider
import androidx.compose.runtime.remember
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.test.ComposeUiTest
import androidx.compose.ui.test.ExperimentalTestApi
import androidx.compose.ui.test.SemanticsNodeInteractionsProvider
import androidx.compose.ui.test.onNodeWithTag
import androidx.compose.ui.test.performMouseInput
import androidx.compose.ui.test.performTouchInput
import androidx.compose.ui.test.runComposeUiTest
import androidx.compose.ui.test.swipe
import androidx.compose.ui.test.swipeWithVelocity
import androidx.compose.ui.unit.dp
import co.typie.ext.horizontalScroll
import co.typie.icons.Lucide
import co.typie.ui.theme.LightAppShadows
import co.typie.ui.theme.LightColors
import co.typie.ui.theme.LocalAppColors
import co.typie.ui.theme.LocalAppShadows
import co.typie.ui.theme.LocalThemeMode
import co.typie.ui.theme.ResolvedThemeMode
import dev.chrisbanes.haze.HazeStyle
import dev.chrisbanes.haze.LocalHazeStyle
import kotlin.math.abs
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertTrue

@OptIn(ExperimentalTestApi::class)
class EditorToolbarPagesDesktopTest {
  @Test
  fun mainPageDragLeftMovesToTextToolbarAtStart() = runComposeUiTest {
    val textScrollState = setToolbarContent()

    goToTextPage()

    assertEquals(
      0,
      textScrollState.value,
      "text toolbar should enter at the start from the previous page",
    )
    assertPageActive(TextPageTag)
  }

  @Test
  fun imagePageDragRightMovesToTextToolbarAtEnd() = runComposeUiTest {
    val textScrollState = setToolbarContent()
    goToImagePage(textScrollState)

    swipeToolbarRight(distanceFraction = 0.7f)

    assertEquals(
      textScrollState.maxValue,
      textScrollState.value,
      "text toolbar should enter at the end from the next page",
    )
    assertPageActive(TextPageTag)
  }

  @Test
  fun slowLongDragFromMainContinuesIntoTextToolbarOverflow() = runComposeUiTest {
    val textScrollState = setToolbarContent()

    swipeToolbarLeft(distanceFraction = 1.8f, durationMillis = 1400)

    assertTrue(
      textScrollState.value > 0,
      "drag from main should continue into text toolbar overflow",
    )
    assertTrue(
      textScrollState.value < textScrollState.maxValue,
      "drag should not exhaust text toolbar overflow",
    )
    assertPageActive(TextPageTag)
  }

  @Test
  fun veryLongDragFromMainStopsAtTextToolbarEnd() = runComposeUiTest {
    val textScrollState = setToolbarContent()

    swipeToolbarLeft(distanceFraction = 4.9f, durationMillis = 500)

    assertEquals(
      textScrollState.maxValue,
      textScrollState.value,
      "drag from main should stop at the text toolbar end",
    )
    assertPageActive(TextPageTag)
  }

  @Test
  fun fineGrainedLongDragFromTextStartStopsAtTextToolbarEnd() = runComposeUiTest {
    val textScrollState = setToolbarContent()
    goToTextPage()

    dragToolbarLeftInSteps(distanceFraction = 4.9f, steps = 120)

    assertEquals(
      textScrollState.maxValue,
      textScrollState.value,
      "fine-grained drag should stop at the text toolbar end",
    )
    assertPageActive(TextPageTag)
  }

  @Test
  fun fineGrainedWheelFromTextStartHardStopsAtTextToolbarEnd() = runComposeUiTest {
    val textScrollState = setToolbarContent()
    goToTextPage()

    repeat(160) { wheelToolbar(delta = 8f) }

    assertEquals(
      textScrollState.maxValue,
      textScrollState.value,
      "fine-grained wheel should hard stop at the text toolbar end",
    )
    assertPageActive(TextPageTag)
  }

  @Test
  fun slowLongDragFromImageContinuesIntoTextToolbarOverflow() = runComposeUiTest {
    val textScrollState = setToolbarContent()
    goToImagePage(textScrollState)

    swipeToolbarRight(distanceFraction = 1.8f, durationMillis = 1400)

    assertTrue(
      textScrollState.value in 1 until textScrollState.maxValue,
      "drag from image should continue into text toolbar overflow",
    )
    assertPageActive(TextPageTag)
  }

  @Test
  fun veryLongDragFromImageStopsAtTextToolbarStart() = runComposeUiTest {
    val textScrollState = setToolbarContent()
    goToImagePage(textScrollState)

    swipeToolbarRight(distanceFraction = 4.9f, durationMillis = 500)

    assertEquals(0, textScrollState.value, "drag from image should stop at the text toolbar start")
    assertPageActive(TextPageTag)
  }

  @Test
  fun textToolbarDragLeftFromStartScrollsInsideTextToolbarBeforeChangingPages() = runComposeUiTest {
    val textScrollState = setToolbarContent()
    goToTextPage()
    val initialTextLeft = pageLeft(TextPageTag)

    swipeToolbarLeft(distanceFraction = 0.3f)

    assertTrue(textScrollState.value > 0, "drag should first move the text toolbar's own scroll")
    assertNear(initialTextLeft, pageLeft(TextPageTag), "text page should stay in place")
  }

  @Test
  fun textToolbarHorizontalWheelScrollsInsideTextToolbarBeforeChangingPages() = runComposeUiTest {
    val textScrollState = setToolbarContent()
    goToTextPage()
    val initialTextLeft = pageLeft(TextPageTag)

    wheelToolbar(delta = 600f)

    assertTrue(
      textScrollState.value > 0,
      "horizontal wheel should move the text toolbar's own scroll",
    )
    assertNear(initialTextLeft, pageLeft(TextPageTag), "text page should stay in place")
  }

  @Test
  fun textToolbarHorizontalWheelAtRightEdgeMovesToImage() = runComposeUiTest {
    val textScrollState = setToolbarContent()
    goToTextPageAtEnd(textScrollState)

    wheelToolbar(delta = 120f)
    wheelToolbar(delta = 120f)

    assertEquals(
      textScrollState.maxValue,
      textScrollState.value,
      "wheel paging should keep text scroll at the end",
    )
    assertPageActive(ImagePageTag)
  }

  @Test
  fun textToolbarDragRightFromMiddleScrollsInsideTextToolbarBeforeChangingPages() =
    runComposeUiTest {
      val textScrollState = setToolbarContent()
      goToTextPage()
      swipeToolbarLeft(distanceFraction = 0.3f, durationMillis = 700)
      val middleScroll = textScrollState.value
      val initialTextLeft = pageLeft(TextPageTag)

      swipeToolbarRight(distanceFraction = 0.15f, durationMillis = 700)

      assertTrue(
        textScrollState.value in 1 until middleScroll,
        "right drag should reduce the text toolbar's own scroll first",
      )
      assertNear(initialTextLeft, pageLeft(TextPageTag), "text page should stay in place")
    }

  @Test
  fun textToolbarLeftEdgeShortDragBouncesWithoutMovingToMain() = runComposeUiTest {
    val textScrollState = setToolbarContent()
    goToTextPage()
    val initialTextLeft = pageLeft(TextPageTag)

    swipeToolbarRight(distanceFraction = 0.16f)

    assertEquals(
      0,
      textScrollState.value,
      "left edge bounce should keep internal scroll at the start",
    )
    assertNear(
      initialTextLeft,
      pageLeft(TextPageTag),
      "short edge drag should stay on the text page",
    )
  }

  @Test
  fun textToolbarLeftEdgeLongDragMovesToMain() = runComposeUiTest {
    val textScrollState = setToolbarContent()
    goToTextPage()

    swipeToolbarRight(distanceFraction = 0.82f)

    assertEquals(
      0,
      textScrollState.value,
      "moving to the previous page should keep text scroll at the start",
    )
    assertPageActive(MainPageTag)
  }

  @Test
  fun textToolbarRightEdgeShortDragBouncesWithoutJumpingToStart() = runComposeUiTest {
    val textScrollState = setToolbarContent()
    goToTextPageAtEnd(textScrollState)
    val maxScroll = textScrollState.maxValue
    val initialTextLeft = pageLeft(TextPageTag)

    swipeToolbarLeft(distanceFraction = 0.17f, durationMillis = 700)

    assertEquals(
      maxScroll,
      textScrollState.value,
      "right edge bounce should keep internal scroll at the end",
    )
    assertNear(
      initialTextLeft,
      pageLeft(TextPageTag),
      "short edge drag should stay on the text page",
    )
  }

  @Test
  fun textToolbarRightEdgeLongDragMovesToImage() = runComposeUiTest {
    val textScrollState = setToolbarContent()
    goToTextPageAtEnd(textScrollState)
    val maxScroll = textScrollState.maxValue

    swipeToolbarLeft(distanceFraction = 0.82f)

    assertEquals(
      maxScroll,
      textScrollState.value,
      "moving to the next page should keep text scroll at the end",
    )
    assertPageActive(ImagePageTag)
  }

  @Test
  fun textToolbarRightEdgeTinyReverseMotionStillMovesToImage() = runComposeUiTest {
    val textScrollState = setToolbarContent()
    goToTextPageAtEnd(textScrollState)
    val maxScroll = textScrollState.maxValue

    dragToolbarWithTinyReverseMotion()

    assertEquals(
      maxScroll,
      textScrollState.value,
      "tiny reverse motion should not turn an edge escape into a hard stop",
    )
    assertPageActive(ImagePageTag)
  }

  @Test
  fun textToolbarRightEdgeReverseThenForwardDragStillMovesToImage() = runComposeUiTest {
    val textScrollState = setToolbarContent()
    goToTextPageAtEnd(textScrollState)
    val maxScroll = textScrollState.maxValue

    dragToolbarRightThenLeftFromRightEdge()

    assertEquals(
      maxScroll,
      textScrollState.value,
      "reverse drag should not turn an edge-start escape into a hard stop",
    )
    assertPageActive(ImagePageTag)
  }

  @Test
  fun textToolbarRightEdgeTinyReverseWheelStillMovesToImage() = runComposeUiTest {
    val textScrollState = setToolbarContent()
    goToTextPageAtEnd(textScrollState)
    val maxScroll = textScrollState.maxValue

    wheelToolbar(delta = -1f)
    wheelToolbar(delta = 120f)

    assertEquals(
      maxScroll,
      textScrollState.value,
      "tiny reverse wheel delta should not turn an edge escape into a hard stop",
    )
    assertPageActive(ImagePageTag)
  }

  @Test
  fun textToolbarOutsideRightEdgeEpsilonWheelStillHardStopsBeforeImage() = runComposeUiTest {
    val textScrollState = setToolbarContent()
    goToTextPageAtEnd(textScrollState)
    val maxScroll = textScrollState.maxValue

    moveTextToolbarRightEdgeOutsideEpsilon(textScrollState)

    wheelToolbar(delta = 120f)

    assertEquals(
      maxScroll,
      textScrollState.value,
      "outside-epsilon internal scroll should hard stop at the text edge",
    )
    assertPageActive(TextPageTag)
  }

  @Test
  fun textToolbarOutsideRightEdgeEpsilonDragStillHardStopsBeforeImage() = runComposeUiTest {
    val textScrollState = setToolbarContent()
    goToTextPageAtEnd(textScrollState)
    val maxScroll = textScrollState.maxValue

    moveTextToolbarRightEdgeOutsideEpsilon(textScrollState)

    swipeToolbarLeft(distanceFraction = 0.82f)

    assertEquals(
      maxScroll,
      textScrollState.value,
      "outside-epsilon drag should hard stop at the text edge",
    )
    assertPageActive(TextPageTag)
  }

  @Test
  fun imageToolbarLeftEdgeDragBouncesWithoutLeavingImage() = runComposeUiTest {
    val textScrollState = setToolbarContent()
    goToImagePage(textScrollState)

    swipeToolbarLeft(distanceFraction = 0.45f)

    assertPageActive(ImagePageTag)
  }

  @Test
  fun slowSwipeAtTextRightEdgeDoesNotChangePage() = runComposeUiTest {
    val textScrollState = setToolbarContent()
    goToTextPageAtEnd(textScrollState)
    val maxScroll = textScrollState.maxValue

    swipeToolbarLeft(distanceFraction = 0.27f, durationMillis = 450)

    assertEquals(
      maxScroll,
      textScrollState.value,
      "slow swipe should stop at the text toolbar edge",
    )
    assertPageActive(TextPageTag)
  }

  @Test
  fun fastSwipeAtTextRightEdgeMovesToImage() = runComposeUiTest {
    val textScrollState = setToolbarContent()
    goToTextPageAtEnd(textScrollState)
    val maxScroll = textScrollState.maxValue

    flingToolbarLeft(distanceFraction = 0.1f)

    assertEquals(
      maxScroll,
      textScrollState.value,
      "fling from the text edge should keep text scroll at the end",
    )
    assertPageActive(ImagePageTag)
  }

  @Test
  fun fastSwipeInsideTextToolbarKeepsIntermediateOverflowPosition() = runComposeUiTest {
    val textScrollState = setToolbarContent()
    goToTextPage()

    flingToolbarLeft(distanceFraction = 0.16f)

    assertTrue(textScrollState.value > 0, "fast swipe should move the text toolbar's own scroll")
    assertTrue(
      textScrollState.value < textScrollState.maxValue,
      "fast swipe inside overflow should not snap to the opposite edge",
    )
    assertPageActive(TextPageTag)
  }

  private fun ComposeUiTest.setToolbarContent(): ScrollState {
    lateinit var textScrollState: ScrollState
    setContent {
      textScrollState = rememberScrollState()
      ToolbarTestContent(textScrollState = textScrollState)
    }
    waitForIdle()
    assertPageActive(MainPageTag)
    return textScrollState
  }

  private fun ComposeUiTest.goToTextPage() {
    swipeToolbarLeft(distanceFraction = 0.7f)
    assertPageActive(TextPageTag)
  }

  private fun ComposeUiTest.goToImagePage(textScrollState: ScrollState) {
    goToTextPageAtEnd(textScrollState)
    swipeToolbarLeft(distanceFraction = 0.82f)
    assertEquals(textScrollState.maxValue, textScrollState.value)
    assertPageActive(ImagePageTag)
  }

  private fun ComposeUiTest.goToTextPageAtEnd(textScrollState: ScrollState) {
    goToTextPage()
    swipeToolbarLeft(distanceFraction = 4.9f, durationMillis = 500)
    assertEquals(textScrollState.maxValue, textScrollState.value)
    assertPageActive(TextPageTag)
  }

  private fun ComposeUiTest.swipeToolbarLeft(distanceFraction: Float, durationMillis: Long = 120) {
    swipeToolbar(
      startFraction = 0.82f,
      endFraction = 0.82f - distanceFraction,
      durationMillis = durationMillis,
    )
  }

  private fun ComposeUiTest.swipeToolbarRight(distanceFraction: Float, durationMillis: Long = 120) {
    swipeToolbar(
      startFraction = 0.18f,
      endFraction = 0.18f + distanceFraction,
      durationMillis = durationMillis,
    )
  }

  private fun ComposeUiTest.flingToolbarLeft(distanceFraction: Float) {
    flingToolbar(startFraction = 0.82f, endFraction = 0.82f - distanceFraction, endVelocity = 1400f)
  }

  private fun ComposeUiTest.wheelToolbar(delta: Float) {
    onNodeWithTag(ToolbarTag).performMouseInput {
      moveTo(Offset(x = width * 0.5f, y = height - 16f))
      scroll(Offset(x = delta, y = 0f))
    }
    waitForIdle()
  }

  private fun ComposeUiTest.dragToolbarWithTinyReverseMotion() {
    onNodeWithTag(ToolbarTag).performTouchInput {
      val start = Offset(x = width * 0.82f, y = height - 16f)
      down(start)
      moveTo(Offset(x = start.x + 1f, y = start.y))
      moveTo(Offset(x = start.x - width * 0.82f, y = start.y))
      up()
    }
    waitForIdle()
  }

  private fun ComposeUiTest.dragToolbarRightThenLeftFromRightEdge() {
    onNodeWithTag(ToolbarTag).performTouchInput {
      val start = Offset(x = width * 0.82f, y = height - 16f)
      val reverse = Offset(x = start.x + width * 0.28f, y = start.y)
      down(start)
      moveTo(reverse)
      moveTo(Offset(x = reverse.x - width * 1.1f, y = start.y))
      up()
    }
    waitForIdle()
  }

  private fun ComposeUiTest.moveTextToolbarRightEdgeOutsideEpsilon(textScrollState: ScrollState) {
    swipeToolbarRight(distanceFraction = 0.16f, durationMillis = 700)

    val remainingScroll = textScrollState.maxValue - textScrollState.value
    assertTrue(
      remainingScroll in 11..48,
      "test setup should leave text toolbar outside the edge epsilon. remaining=$remainingScroll value=${textScrollState.value} max=${textScrollState.maxValue}",
    )
  }

  private fun ComposeUiTest.dragToolbarLeftInSteps(distanceFraction: Float, steps: Int) {
    onNodeWithTag(ToolbarTag).performTouchInput {
      val start = Offset(x = width * 0.82f, y = height - 16f)
      down(start)
      repeat(steps) { step ->
        val progress = (step + 1).toFloat() / steps
        moveTo(Offset(x = start.x - width * distanceFraction * progress, y = start.y))
      }
      up()
    }
    waitForIdle()
  }

  private fun ComposeUiTest.swipeToolbar(
    startFraction: Float,
    endFraction: Float,
    durationMillis: Long,
  ) {
    onNodeWithTag(ToolbarTag).performTouchInput {
      swipe(
        start = Offset(x = width * startFraction, y = height - 16f),
        end = Offset(x = width * endFraction, y = height - 16f),
        durationMillis = durationMillis,
      )
    }
    waitForIdle()
  }

  private fun ComposeUiTest.flingToolbar(
    startFraction: Float,
    endFraction: Float,
    endVelocity: Float,
  ) {
    onNodeWithTag(ToolbarTag).performTouchInput {
      swipeWithVelocity(
        start = Offset(x = width * startFraction, y = height - 16f),
        end = Offset(x = width * endFraction, y = height - 16f),
        endVelocity = endVelocity,
      )
    }
    waitForIdle()
  }

  private fun SemanticsNodeInteractionsProvider.assertPageActive(tag: String) {
    assertNear(0f, pageLeft(tag), "$tag should be the active page")
  }

  private fun SemanticsNodeInteractionsProvider.pageLeft(tag: String): Float =
    onNodeWithTag(tag).fetchSemanticsNode().boundsInRoot.left

  private fun assertNear(expected: Float, actual: Float, message: String) {
    assertTrue(abs(expected - actual) <= 2f, "$message. expected=$expected actual=$actual")
  }

  @Composable
  private fun ToolbarTestContent(textScrollState: ScrollState) {
    val pages = rememberToolbarTestPages(textScrollState)
    ToolbarTestTheme {
      Box(Modifier.width(360.dp).height(ToolbarStackHeight).testTag(ToolbarTag)) {
        EditorToolbarPages(
          pages = pages,
          editorFocused = true,
          activeBottomPanel = null,
          keyboardType = EditorKeyboardType.Software,
          softwareKeyboardVisible = true,
          onEditorInputRequest = {},
          onKeyboardDismissRequest = {},
          onBottomPanelToggle = {},
          modifier = Modifier.fillMaxSize(),
        )
      }
    }
  }

  @Composable
  private fun ToolbarTestTheme(content: @Composable () -> Unit) {
    CompositionLocalProvider(
      LocalAppColors provides LightColors,
      LocalAppShadows provides LightAppShadows,
      LocalThemeMode provides ResolvedThemeMode.Light,
      LocalHazeStyle provides HazeStyle(blurRadius = 20.dp, noiseFactor = 0f, tints = listOf()),
      content = content,
    )
  }

  @Composable
  private fun rememberToolbarTestPages(textScrollState: ScrollState): List<EditorToolbarPage> =
    remember(textScrollState) {
      listOf(
        EditorToolbarPage(
          key = EditorToolbarPageKey.Main,
          icon = Lucide.CircleSmall,
          contentDescription = "메인 툴바",
          content = { Box(Modifier.fillMaxSize().testTag(MainPageTag)) },
        ),
        EditorToolbarPage(
          key = EditorToolbarPageKey.Text,
          icon = Lucide.Type,
          contentDescription = "텍스트 툴바",
          scrollState = textScrollState,
          content = { scope ->
            Row(
              modifier =
                Modifier.fillMaxSize()
                  .testTag(TextPageTag)
                  .horizontalScroll(textScrollState, enabled = false),
              verticalAlignment = Alignment.CenterVertically,
              horizontalArrangement = Arrangement.spacedBy(ToolbarItemGap),
            ) {
              repeat(16) { index ->
                Box(
                  Modifier.size(width = 80.dp, height = ToolbarButtonSize)
                    .testTag("text-item-$index")
                )
              }
              if (scope.hasNextPage) {
                EditorToolbarPageIndicator()
              }
            }
          },
        ),
        EditorToolbarPage(
          key = EditorToolbarPageKey.Image,
          icon = Lucide.Image,
          contentDescription = "이미지 툴바",
          content = { Box(Modifier.fillMaxSize().testTag(ImagePageTag)) },
        ),
      )
    }

  private companion object {
    const val ToolbarTag = "editor-toolbar"
    const val MainPageTag = "main-page"
    const val TextPageTag = "text-page"
    const val ImagePageTag = "image-page"
  }
}
