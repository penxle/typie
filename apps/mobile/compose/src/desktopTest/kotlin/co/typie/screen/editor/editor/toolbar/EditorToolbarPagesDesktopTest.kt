package co.typie.screen.editor.editor.toolbar

import androidx.compose.foundation.ScrollState
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.rememberScrollState
import androidx.compose.runtime.Composable
import androidx.compose.runtime.CompositionLocalProvider
import androidx.compose.runtime.MutableState
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.hapticfeedback.HapticFeedback
import androidx.compose.ui.hapticfeedback.HapticFeedbackType
import androidx.compose.ui.input.pointer.PointerEventPass
import androidx.compose.ui.input.pointer.pointerInput
import androidx.compose.ui.platform.LocalDensity
import androidx.compose.ui.platform.LocalHapticFeedback
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.test.ComposeUiTest
import androidx.compose.ui.test.ExperimentalTestApi
import androidx.compose.ui.test.SemanticsNodeInteractionsProvider
import androidx.compose.ui.test.click
import androidx.compose.ui.test.onNodeWithContentDescription
import androidx.compose.ui.test.onNodeWithTag
import androidx.compose.ui.test.performMouseInput
import androidx.compose.ui.test.performTouchInput
import androidx.compose.ui.test.runComposeUiTest
import androidx.compose.ui.test.swipe
import androidx.compose.ui.test.swipeWithVelocity
import androidx.compose.ui.unit.Dp
import androidx.compose.ui.unit.dp
import co.typie.ext.horizontalScroll
import co.typie.icons.Lucide
import co.typie.ui.theme.LightAppShadows
import co.typie.ui.theme.LightColors
import co.typie.ui.theme.LocalAppColors
import co.typie.ui.theme.LocalAppShadows
import co.typie.ui.theme.LocalThemeMode
import co.typie.ui.theme.ResolvedThemeMode
import dev.chrisbanes.haze.blur.HazeBlurStyle
import dev.chrisbanes.haze.blur.LocalHazeBlurStyle
import kotlin.math.abs
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertFalse
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
  fun outerDragMovesToolbarAtBothEndsAndReturnsOnRelease() = runComposeUiTest {
    val textScrollState = setToolbarContent()
    val initialLeft = pageLeft(MainPageTag)

    startToolbarDrag(startFraction = 0.18f, endFraction = 0.58f)

    assertTrue(pageLeft(MainPageTag) > initialLeft)

    releaseToolbarDrag()

    assertNear(initialLeft, pageLeft(MainPageTag), "release should return the outer offset")
    goToImagePage(textScrollState)
    val initialRight = pageRight(ImagePageTag)

    startToolbarDrag(startFraction = 0.82f, endFraction = 0.42f)

    assertTrue(pageRight(ImagePageTag) < initialRight)

    releaseToolbarDrag()

    assertNear(initialRight, pageRight(ImagePageTag), "release should return the outer offset")
  }

  @Test
  fun outerHorizontalWheelDoesNotCreateDragOverscroll() = runComposeUiTest {
    val textScrollState = setToolbarContent()
    val initialLeft = pageLeft(MainPageTag)

    wheelToolbar(delta = -120f)

    assertPageActive(MainPageTag)
    assertNear(initialLeft, pageLeft(MainPageTag), "wheel should not move the first outer edge")

    goToImagePage(textScrollState)
    val initialRight = pageRight(ImagePageTag)
    wheelToolbar(delta = 120f)

    assertPageActive(ImagePageTag)
    assertNear(initialRight, pageRight(ImagePageTag), "wheel should not move the last outer edge")
  }

  @Test
  fun newOuterDragCancelsReturnAnimation() = runComposeUiTest {
    setToolbarContent()
    val initialLeft = pageLeft(MainPageTag)
    startToolbarDrag(startFraction = 0.18f, endFraction = 0.58f)
    assertTrue(pageLeft(MainPageTag) > initialLeft)

    mainClock.autoAdvance = false
    onNodeWithTag(ToolbarTag).performTouchInput { up() }
    mainClock.advanceTimeByFrame()
    onNodeWithTag(ToolbarTag).performTouchInput {
      down(Offset(x = width * 0.18f, y = height - 16f))
      moveTo(Offset(x = width * 0.58f, y = height - 16f))
    }
    mainClock.advanceTimeByFrame()
    val heldLeft = pageLeft(MainPageTag)
    assertTrue(heldLeft > initialLeft)

    mainClock.advanceTimeBy(200)

    assertNear(heldLeft, pageLeft(MainPageTag), "previous return should not overwrite a new drag")

    onNodeWithTag(ToolbarTag).performTouchInput { up() }
    mainClock.advanceTimeByFrame()
    mainClock.autoAdvance = true
    waitForIdle()

    assertNear(initialLeft, pageLeft(MainPageTag), "released second drag should return to rest")
  }

  @Test
  fun outerDragDoesNotSurviveToolbarDisposalDuringReturn() = runComposeUiTest {
    val toolbarVisible = mutableStateOf(true)
    setContent {
      val pagerState = rememberToolbarPagerState()
      val textScrollState = rememberScrollState()
      if (toolbarVisible.value) {
        ToolbarTestContent(textScrollState = textScrollState, pagerState = pagerState)
      }
    }
    waitForIdle()
    val initialLeft = pageLeft(MainPageTag)
    startToolbarDrag(startFraction = 0.18f, endFraction = 0.58f)
    assertTrue(pageLeft(MainPageTag) > initialLeft)

    mainClock.autoAdvance = false
    onNodeWithTag(ToolbarTag).performTouchInput { up() }
    mainClock.advanceTimeByFrame()
    runOnIdle { toolbarVisible.value = false }
    mainClock.advanceTimeByFrame()
    runOnIdle { toolbarVisible.value = true }
    mainClock.advanceTimeByFrame()

    assertNear(initialLeft, pageLeft(MainPageTag), "disposed drag offset should not return")
    mainClock.autoAdvance = true
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

  @Test
  fun autoTargetMovesToTextPage() = runComposeUiTest {
    val pageKeys = mutableStateOf(DefaultPageKeys)
    val autoTarget = mutableStateOf<EditorToolbarPageKey?>(null)
    val autoTargetRevision = mutableStateOf(0L)
    setDynamicToolbarContent(pageKeys, autoTarget, autoTargetRevision)

    autoTarget.value = EditorToolbarPageKey.Text
    autoTargetRevision.value++
    waitForIdle()

    assertPageActive(TextPageTag)
  }

  @Test
  fun manualNavigationAwayFromAutoTargetSurvivesToolbarRecompose() = runComposeUiTest {
    val pageKeys = mutableStateOf(listOf(EditorToolbarPageKey.Main, EditorToolbarPageKey.Image))
    val autoTarget = mutableStateOf<EditorToolbarPageKey?>(null)
    val autoTargetRevision = mutableStateOf(0L)
    val visible = mutableStateOf(true)
    lateinit var pagerState: ToolbarPagerState
    setContent {
      val textScrollState = rememberScrollState()
      pagerState = rememberToolbarPagerState()
      ToolbarTestContent(
        textScrollState = textScrollState,
        pageKeys = pageKeys.value,
        autoTargetPageKey = autoTarget.value,
        autoTargetRevision = autoTargetRevision.value,
        visible = visible.value,
        pagerState = pagerState,
      )
    }
    waitForIdle()
    assertPageActive(MainPageTag)

    autoTarget.value = EditorToolbarPageKey.Image
    autoTargetRevision.value++
    waitForIdle()
    assertPageActive(ImagePageTag)

    swipeToolbarRight(distanceFraction = 0.7f)
    assertPageActive(MainPageTag)
    assertEquals(EditorToolbarPageKey.Main, pagerState.settledPageKey)

    visible.value = false
    waitForIdle()
    visible.value = true
    waitForIdle()

    assertPageActive(MainPageTag)
    assertEquals(EditorToolbarPageKey.Main, pagerState.settledPageKey)
  }

  @Test
  fun manualIndicatorNavigationBackToAutoTargetStaysOnTarget() = runComposeUiTest {
    val pageKeys = mutableStateOf(listOf(EditorToolbarPageKey.Main, EditorToolbarPageKey.Image))
    val autoTarget = mutableStateOf<EditorToolbarPageKey?>(null)
    val autoTargetRevision = mutableStateOf(0L)
    lateinit var pagerState: ToolbarPagerState
    setContent {
      val textScrollState = rememberScrollState()
      pagerState = rememberToolbarPagerState()
      ToolbarTestContent(
        textScrollState = textScrollState,
        pageKeys = pageKeys.value,
        autoTargetPageKey = autoTarget.value,
        autoTargetRevision = autoTargetRevision.value,
        pagerState = pagerState,
      )
    }
    waitForIdle()

    autoTarget.value = EditorToolbarPageKey.Image
    autoTargetRevision.value++
    waitForIdle()
    assertPageActive(ImagePageTag)

    swipeToolbarRight(distanceFraction = 0.7f)
    assertPageActive(MainPageTag)

    mainClock.autoAdvance = false
    pagerState.indicatorPulse++
    mainClock.advanceTimeByFrame()
    tapToolbarIndicatorPage(pageIndex = 1, pageCount = 2)
    mainClock.autoAdvance = true
    waitForIdle()

    assertPageActive(ImagePageTag)
    assertEquals(EditorToolbarPageKey.Image, pagerState.settledPageKey)
  }

  @Test
  fun manualIndicatorNavigationBackToAutoTargetWithTextPageStaysOnTarget() = runComposeUiTest {
    val pageKeys = mutableStateOf(DefaultPageKeys)
    val autoTarget = mutableStateOf<EditorToolbarPageKey?>(null)
    val autoTargetRevision = mutableStateOf(0L)
    lateinit var pagerState: ToolbarPagerState
    setContent {
      val textScrollState = rememberScrollState()
      pagerState = rememberToolbarPagerState()
      ToolbarTestContent(
        textScrollState = textScrollState,
        pageKeys = pageKeys.value,
        autoTargetPageKey = autoTarget.value,
        autoTargetRevision = autoTargetRevision.value,
        pagerState = pagerState,
      )
    }
    waitForIdle()

    autoTarget.value = EditorToolbarPageKey.Image
    autoTargetRevision.value++
    waitForIdle()
    assertPageActive(ImagePageTag)

    swipeToolbarRight(distanceFraction = 0.7f)
    assertPageActive(TextPageTag)
    swipeToolbarRight(distanceFraction = 4.9f, durationMillis = 500)
    assertPageActive(TextPageTag)
    swipeToolbarRight(distanceFraction = 0.82f)
    assertPageActive(MainPageTag)

    mainClock.autoAdvance = false
    pagerState.indicatorPulse++
    mainClock.advanceTimeByFrame()
    tapToolbarIndicatorPage(pageIndex = 2, pageCount = 3)
    mainClock.autoAdvance = true
    waitForIdle()

    assertPageActive(ImagePageTag)
    assertEquals(EditorToolbarPageKey.Image, pagerState.settledPageKey)
  }

  @Test
  fun disappearingCurrentPageMovesToMain() = runComposeUiTest {
    val pageKeys = mutableStateOf(DefaultPageKeys)
    val autoTarget = mutableStateOf<EditorToolbarPageKey?>(null)
    val autoTargetRevision = mutableStateOf(0L)
    val textScrollState = setDynamicToolbarContent(pageKeys, autoTarget, autoTargetRevision)
    goToImagePage(textScrollState)

    pageKeys.value = listOf(EditorToolbarPageKey.Main, EditorToolbarPageKey.Text)
    autoTargetRevision.value++
    waitForIdle()

    assertPageActive(MainPageTag)
  }

  @Test
  fun disappearingAutoTargetPrefersRecentManualPage() = runComposeUiTest {
    val pageKeys = mutableStateOf(listOf(EditorToolbarPageKey.Main, EditorToolbarPageKey.Text))
    val autoTarget = mutableStateOf<EditorToolbarPageKey?>(null)
    val autoTargetRevision = mutableStateOf(0L)
    setDynamicToolbarContent(pageKeys, autoTarget, autoTargetRevision)
    goToTextPage()

    pageKeys.value = DefaultPageKeys + listOf(EditorToolbarPageKey.File)
    autoTarget.value = EditorToolbarPageKey.File
    autoTargetRevision.value++
    waitForIdle()
    assertPageActive(FilePageTag)

    pageKeys.value = DefaultPageKeys
    autoTarget.value = null
    autoTargetRevision.value++
    waitForIdle()

    assertPageActive(TextPageTag)
  }

  @Test
  fun disappearingManualContextPageFallsBackToPreviousManualPage() = runComposeUiTest {
    val pageKeys = mutableStateOf(listOf(EditorToolbarPageKey.Main, EditorToolbarPageKey.Text))
    val autoTarget = mutableStateOf<EditorToolbarPageKey?>(null)
    val autoTargetRevision = mutableStateOf(0L)
    val textScrollState = setDynamicToolbarContent(pageKeys, autoTarget, autoTargetRevision)
    goToTextPage()

    pageKeys.value =
      listOf(EditorToolbarPageKey.Main, EditorToolbarPageKey.Text, EditorToolbarPageKey.Blockquote)
    autoTargetRevision.value++
    waitForIdle()
    assertPageActive(TextPageTag)

    goToBlockquotePage(textScrollState)

    pageKeys.value = listOf(EditorToolbarPageKey.Main, EditorToolbarPageKey.Text)
    autoTargetRevision.value++
    waitForIdle()

    assertPageActive(TextPageTag)
  }

  @Test
  fun returningToManuallySelectedBlockquoteContextRestoresBlockquotePage() = runComposeUiTest {
    val pageKeys = mutableStateOf(listOf(EditorToolbarPageKey.Main, EditorToolbarPageKey.Text))
    val autoTarget = mutableStateOf<EditorToolbarPageKey?>(null)
    val autoTargetRevision = mutableStateOf(0L)
    val textScrollState = setDynamicToolbarContent(pageKeys, autoTarget, autoTargetRevision)
    goToTextPage()

    pageKeys.value =
      listOf(EditorToolbarPageKey.Main, EditorToolbarPageKey.Text, EditorToolbarPageKey.Blockquote)
    autoTargetRevision.value++
    waitForIdle()
    goToBlockquotePage(textScrollState)

    pageKeys.value = listOf(EditorToolbarPageKey.Main, EditorToolbarPageKey.Text)
    autoTargetRevision.value++
    waitForIdle()
    assertPageActive(TextPageTag)

    pageKeys.value =
      listOf(EditorToolbarPageKey.Main, EditorToolbarPageKey.Text, EditorToolbarPageKey.Blockquote)
    autoTargetRevision.value++
    waitForIdle()

    autoTargetRevision.value++
    waitForIdle()

    assertPageActive(BlockquotePageTag)
  }

  @Test
  fun returningToBlockquoteAfterAutoTextRestoresManualBlockquotePage() = runComposeUiTest {
    val pageKeys = mutableStateOf(listOf(EditorToolbarPageKey.Main, EditorToolbarPageKey.Text))
    val autoTarget = mutableStateOf<EditorToolbarPageKey?>(null)
    val autoTargetRevision = mutableStateOf(0L)
    val textScrollState = setDynamicToolbarContent(pageKeys, autoTarget, autoTargetRevision)
    goToTextPage()

    pageKeys.value =
      listOf(EditorToolbarPageKey.Main, EditorToolbarPageKey.Text, EditorToolbarPageKey.Blockquote)
    autoTargetRevision.value++
    waitForIdle()
    goToBlockquotePage(textScrollState)

    pageKeys.value = listOf(EditorToolbarPageKey.Main, EditorToolbarPageKey.Text)
    autoTarget.value = EditorToolbarPageKey.Text
    autoTargetRevision.value++
    waitForIdle()
    assertPageActive(TextPageTag)

    pageKeys.value =
      listOf(EditorToolbarPageKey.Main, EditorToolbarPageKey.Text, EditorToolbarPageKey.Blockquote)
    autoTarget.value = null
    autoTargetRevision.value++
    waitForIdle()

    assertPageActive(BlockquotePageTag)
  }

  @Test
  fun returningToBlockquoteAfterAutoTextClearsOutsideRestoresManualBlockquotePage() =
    runComposeUiTest {
      val pageKeys = mutableStateOf(listOf(EditorToolbarPageKey.Main, EditorToolbarPageKey.Text))
      val autoTarget = mutableStateOf<EditorToolbarPageKey?>(null)
      val autoTargetRevision = mutableStateOf(0L)
      val textScrollState = setDynamicToolbarContent(pageKeys, autoTarget, autoTargetRevision)
      goToTextPage()

      pageKeys.value =
        listOf(
          EditorToolbarPageKey.Main,
          EditorToolbarPageKey.Text,
          EditorToolbarPageKey.Blockquote,
        )
      autoTargetRevision.value++
      waitForIdle()
      goToBlockquotePage(textScrollState)

      pageKeys.value = listOf(EditorToolbarPageKey.Main, EditorToolbarPageKey.Text)
      autoTarget.value = EditorToolbarPageKey.Text
      autoTargetRevision.value++
      waitForIdle()
      assertPageActive(TextPageTag)

      autoTarget.value = null
      autoTargetRevision.value++
      waitForIdle()
      assertPageActive(TextPageTag)

      pageKeys.value =
        listOf(
          EditorToolbarPageKey.Main,
          EditorToolbarPageKey.Text,
          EditorToolbarPageKey.Blockquote,
        )
      autoTargetRevision.value++
      waitForIdle()

      autoTargetRevision.value++
      waitForIdle()

      assertPageActive(BlockquotePageTag)
    }

  @Test
  fun returningToBlockquoteWhileAutoTextActiveRestoresManualBlockquotePage() = runComposeUiTest {
    val pageKeys = mutableStateOf(listOf(EditorToolbarPageKey.Main, EditorToolbarPageKey.Text))
    val autoTarget = mutableStateOf<EditorToolbarPageKey?>(null)
    val autoTargetRevision = mutableStateOf(0L)
    val textScrollState = setDynamicToolbarContent(pageKeys, autoTarget, autoTargetRevision)
    goToTextPage()

    pageKeys.value =
      listOf(EditorToolbarPageKey.Main, EditorToolbarPageKey.Text, EditorToolbarPageKey.Blockquote)
    autoTargetRevision.value++
    waitForIdle()
    goToBlockquotePage(textScrollState)

    pageKeys.value = listOf(EditorToolbarPageKey.Main, EditorToolbarPageKey.Text)
    autoTarget.value = EditorToolbarPageKey.Text
    autoTargetRevision.value++
    waitForIdle()
    assertPageActive(TextPageTag)

    pageKeys.value =
      listOf(EditorToolbarPageKey.Main, EditorToolbarPageKey.Text, EditorToolbarPageKey.Blockquote)
    autoTargetRevision.value++
    waitForIdle()

    assertPageActive(BlockquotePageTag)
  }

  @Test
  fun autoTextAfterContextFallbackDoesNotReplaceLastManualContextPage() = runComposeUiTest {
    val pageKeys = mutableStateOf(listOf(EditorToolbarPageKey.Main, EditorToolbarPageKey.Text))
    val autoTarget = mutableStateOf<EditorToolbarPageKey?>(null)
    val autoTargetRevision = mutableStateOf(0L)
    val textScrollState = setDynamicToolbarContent(pageKeys, autoTarget, autoTargetRevision)
    goToTextPage()

    pageKeys.value =
      listOf(EditorToolbarPageKey.Main, EditorToolbarPageKey.Text, EditorToolbarPageKey.Blockquote)
    autoTargetRevision.value++
    waitForIdle()
    goToBlockquotePage(textScrollState)

    pageKeys.value = listOf(EditorToolbarPageKey.Main, EditorToolbarPageKey.Text)
    autoTargetRevision.value++
    waitForIdle()
    assertPageActive(TextPageTag)

    autoTarget.value = EditorToolbarPageKey.Text
    autoTargetRevision.value++
    waitForIdle()
    assertPageActive(TextPageTag)

    pageKeys.value =
      listOf(EditorToolbarPageKey.Main, EditorToolbarPageKey.Text, EditorToolbarPageKey.Blockquote)
    autoTarget.value = null
    autoTargetRevision.value++
    waitForIdle()

    assertPageActive(BlockquotePageTag)
  }

  @Test
  fun automaticTransitionToTextResetsTextToolbarScroll() = runComposeUiTest {
    val pageKeys = mutableStateOf(DefaultPageKeys)
    val autoTarget = mutableStateOf<EditorToolbarPageKey?>(null)
    val autoTargetRevision = mutableStateOf(0L)
    val textScrollState = setDynamicToolbarContent(pageKeys, autoTarget, autoTargetRevision)
    goToImagePage(textScrollState)

    autoTarget.value = EditorToolbarPageKey.Text
    autoTargetRevision.value++
    waitForIdle()

    assertEquals(0, textScrollState.value)
    assertPageActive(TextPageTag)
  }

  @Test
  fun retainedTextPageKeepsInternalScrollWhenPageKeysChange() = runComposeUiTest {
    val pageKeys = mutableStateOf(DefaultPageKeys)
    val autoTarget = mutableStateOf<EditorToolbarPageKey?>(null)
    val autoTargetRevision = mutableStateOf(0L)
    val textScrollState = setDynamicToolbarContent(pageKeys, autoTarget, autoTargetRevision)
    goToTextPageAtEnd(textScrollState)
    val textScroll = textScrollState.value

    pageKeys.value =
      listOf(EditorToolbarPageKey.Main, EditorToolbarPageKey.Text, EditorToolbarPageKey.Blockquote)
    autoTargetRevision.value++
    waitForIdle()

    assertEquals(textScroll, textScrollState.value)
    assertPageActive(TextPageTag)
  }

  @Test
  fun clearedAutoTargetRestoresRecentManualPageWhenAutoPageStillAvailable() = runComposeUiTest {
    val pageKeys =
      mutableStateOf(
        listOf(EditorToolbarPageKey.Main, EditorToolbarPageKey.Text, EditorToolbarPageKey.File)
      )
    val autoTarget = mutableStateOf<EditorToolbarPageKey?>(null)
    val autoTargetRevision = mutableStateOf(0L)
    setDynamicToolbarContent(pageKeys, autoTarget, autoTargetRevision)
    goToTextPage()

    autoTarget.value = EditorToolbarPageKey.File
    autoTargetRevision.value++
    waitForIdle()
    assertPageActive(FilePageTag)

    autoTarget.value = null
    autoTargetRevision.value++
    waitForIdle()

    assertPageActive(TextPageTag)
  }

  @Test
  fun clearedAutoTargetKeepsManualPageWhenStillAvailable() = runComposeUiTest {
    val pageKeys =
      mutableStateOf(
        listOf(EditorToolbarPageKey.Main, EditorToolbarPageKey.Text, EditorToolbarPageKey.Fold)
      )
    val autoTarget = mutableStateOf<EditorToolbarPageKey?>(null)
    val autoTargetRevision = mutableStateOf(0L)
    val textScrollState = setDynamicToolbarContent(pageKeys, autoTarget, autoTargetRevision)
    goToTextPage()
    goToFoldPage(textScrollState)

    autoTarget.value = EditorToolbarPageKey.Fold
    autoTargetRevision.value++
    waitForIdle()
    assertPageActive(FoldPageTag)

    autoTarget.value = null
    autoTargetRevision.value++
    waitForIdle()

    assertPageActive(FoldPageTag)
  }

  @Test
  fun clearedAutoTargetFromFoldRestoresRecentManualTextPage() = runComposeUiTest {
    val pageKeys =
      mutableStateOf(
        listOf(EditorToolbarPageKey.Main, EditorToolbarPageKey.Text, EditorToolbarPageKey.Fold)
      )
    val autoTarget = mutableStateOf<EditorToolbarPageKey?>(null)
    val autoTargetRevision = mutableStateOf(0L)
    setDynamicToolbarContent(pageKeys, autoTarget, autoTargetRevision)
    goToTextPage()

    autoTarget.value = EditorToolbarPageKey.Fold
    autoTargetRevision.value++
    waitForIdle()
    assertPageActive(FoldPageTag)

    autoTarget.value = null
    autoTargetRevision.value++
    waitForIdle()

    assertPageActive(TextPageTag)
  }

  @Test
  fun returningFromFoldTitleRestoresRecentManualTextPage() = runComposeUiTest {
    val pageKeys =
      mutableStateOf(
        listOf(EditorToolbarPageKey.Main, EditorToolbarPageKey.Text, EditorToolbarPageKey.Fold)
      )
    val autoTarget = mutableStateOf<EditorToolbarPageKey?>(null)
    val autoTargetRevision = mutableStateOf(0L)
    setDynamicToolbarContent(pageKeys, autoTarget, autoTargetRevision)
    goToTextPage()

    pageKeys.value = listOf(EditorToolbarPageKey.Main, EditorToolbarPageKey.Fold)
    autoTarget.value = EditorToolbarPageKey.Fold
    autoTargetRevision.value++
    waitForIdle()
    assertPageActive(FoldPageTag)

    pageKeys.value =
      listOf(EditorToolbarPageKey.Main, EditorToolbarPageKey.Text, EditorToolbarPageKey.Fold)
    autoTarget.value = null
    autoTargetRevision.value++
    waitForIdle()

    assertPageActive(TextPageTag)
  }

  @Test
  fun returningFromFoldTitleFallbackRestoresMostRecentManualTextPage() = runComposeUiTest {
    val pageKeys =
      mutableStateOf(
        listOf(EditorToolbarPageKey.Main, EditorToolbarPageKey.Text, EditorToolbarPageKey.Fold)
      )
    val autoTarget = mutableStateOf<EditorToolbarPageKey?>(null)
    val autoTargetRevision = mutableStateOf(0L)
    val textScrollState = setDynamicToolbarContent(pageKeys, autoTarget, autoTargetRevision)
    goToTextPage()
    goToFoldPage(textScrollState)
    swipeToolbarRight(distanceFraction = 0.82f)
    assertPageActive(TextPageTag)

    pageKeys.value = listOf(EditorToolbarPageKey.Main, EditorToolbarPageKey.Fold)
    autoTargetRevision.value++
    waitForIdle()
    assertPageActive(FoldPageTag)

    autoTargetRevision.value++
    waitForIdle()
    assertPageActive(FoldPageTag)

    pageKeys.value =
      listOf(EditorToolbarPageKey.Main, EditorToolbarPageKey.Text, EditorToolbarPageKey.Fold)
    autoTargetRevision.value++
    waitForIdle()

    assertPageActive(TextPageTag)
  }

  @Test
  fun clearedAutoTargetUsesLatestExistingManualPageAfterAutoPageSettlesAgain() = runComposeUiTest {
    val pageKeys =
      mutableStateOf(
        listOf(EditorToolbarPageKey.Main, EditorToolbarPageKey.Text, EditorToolbarPageKey.Fold)
      )
    val autoTarget = mutableStateOf<EditorToolbarPageKey?>(null)
    val autoTargetRevision = mutableStateOf(0L)
    setDynamicToolbarContent(pageKeys, autoTarget, autoTargetRevision)
    goToTextPage()

    autoTarget.value = EditorToolbarPageKey.Fold
    autoTargetRevision.value++
    waitForIdle()
    assertPageActive(FoldPageTag)

    swipeToolbarLeft(distanceFraction = 0.08f)
    assertPageActive(FoldPageTag)

    autoTarget.value = null
    autoTargetRevision.value++
    waitForIdle()

    assertPageActive(FoldPageTag)
  }

  @Test
  fun interruptedManualTextNavigationIsRememberedAfterAutoFoldClears() = runComposeUiTest {
    val pageKeys =
      mutableStateOf(
        listOf(EditorToolbarPageKey.Main, EditorToolbarPageKey.Text, EditorToolbarPageKey.Fold)
      )
    val autoTarget = mutableStateOf<EditorToolbarPageKey?>(null)
    val autoTargetRevision = mutableStateOf(0L)
    val textScrollState = setDynamicToolbarContent(pageKeys, autoTarget, autoTargetRevision)
    goToFoldPage(textScrollState)

    mainClock.autoAdvance = false
    swipeToolbarRightWithoutIdle(distanceFraction = 0.82f)

    autoTarget.value = EditorToolbarPageKey.Fold
    autoTargetRevision.value++
    mainClock.advanceTimeByFrame()

    autoTarget.value = null
    autoTargetRevision.value++
    mainClock.autoAdvance = true
    waitForIdle()

    assertPageActive(TextPageTag)
  }

  @Test
  fun retainedPageKeySurvivesIndexChange() = runComposeUiTest {
    val pageKeys = mutableStateOf(DefaultPageKeys)
    val autoTarget = mutableStateOf<EditorToolbarPageKey?>(null)
    val autoTargetRevision = mutableStateOf(0L)
    val textScrollState = setDynamicToolbarContent(pageKeys, autoTarget, autoTargetRevision)
    goToImagePage(textScrollState)

    pageKeys.value = listOf(EditorToolbarPageKey.Main, EditorToolbarPageKey.Image)
    autoTargetRevision.value++
    waitForIdle()

    assertPageActive(ImagePageTag)
  }

  @Test
  fun retainedPageKeySnapsWhenItsIndexChanges() = runComposeUiTest {
    val pageKeys = mutableStateOf(DefaultPageKeys)
    val autoTarget = mutableStateOf<EditorToolbarPageKey?>(null)
    val autoTargetRevision = mutableStateOf(0L)
    val textScrollState = setDynamicToolbarContent(pageKeys, autoTarget, autoTargetRevision)
    goToImagePage(textScrollState)

    mainClock.autoAdvance = false
    pageKeys.value = listOf(EditorToolbarPageKey.Main, EditorToolbarPageKey.Image)
    autoTargetRevision.value++
    mainClock.advanceTimeByFrame()

    assertPageActive(ImagePageTag)

    mainClock.autoAdvance = true
    waitForIdle()
  }

  @Test
  fun retainedManualFoldPageStaysFixedWhenTextToolbarExtentChanges() = runComposeUiTest {
    val pageKeys =
      mutableStateOf(
        listOf(EditorToolbarPageKey.Main, EditorToolbarPageKey.Text, EditorToolbarPageKey.Fold)
      )
    val autoTarget = mutableStateOf<EditorToolbarPageKey?>(null)
    val autoTargetRevision = mutableStateOf(0L)
    val textItemWidth = mutableStateOf(80.dp)
    lateinit var textScrollState: ScrollState
    setContent {
      textScrollState = rememberScrollState()
      ToolbarTestContent(
        textScrollState = textScrollState,
        pageKeys = pageKeys.value,
        autoTargetPageKey = autoTarget.value,
        autoTargetRevision = autoTargetRevision.value,
        textItemWidth = textItemWidth.value,
      )
    }
    waitForIdle()
    goToFoldPage(textScrollState)
    val initialTextMax = textScrollState.maxValue
    val initialFoldLeft = pageLeft(FoldPageTag)

    mainClock.autoAdvance = false
    textItemWidth.value = 84.dp
    mainClock.advanceTimeByFrame()

    assertTrue(
      textScrollState.maxValue > initialTextMax,
      "test setup should increase text toolbar extent",
    )
    assertNear(initialFoldLeft, pageLeft(FoldPageTag), "fold page should stay in place")

    mainClock.autoAdvance = true
    waitForIdle()
    assertPageActive(FoldPageTag)
  }

  @Test
  fun textToolbarExtentChangeDoesNotPulseIndicator() = runComposeUiTest {
    val textItemWidth = mutableStateOf(80.dp)
    lateinit var pagerState: ToolbarPagerState
    lateinit var textScrollState: ScrollState
    setContent {
      textScrollState = rememberScrollState()
      pagerState = rememberToolbarPagerState()
      ToolbarTestContent(
        textScrollState = textScrollState,
        pagerState = pagerState,
        textItemWidth = textItemWidth.value,
      )
    }
    waitForIdle()
    val initialTextMax = textScrollState.maxValue
    val initialPulse = pagerState.indicatorPulse

    textItemWidth.value = 84.dp
    waitForIdle()

    assertTrue(
      textScrollState.maxValue > initialTextMax,
      "test setup should increase text toolbar extent",
    )
    assertEquals(
      initialPulse,
      pagerState.indicatorPulse,
      "indicator should not pulse when only a page scroll range changes",
    )
  }

  @Test
  fun retainedPagerStateRestoresPageAndInternalScrollAfterToolbarRecomposes() = runComposeUiTest {
    val visible = mutableStateOf(true)
    val pageKeys = mutableStateOf(DefaultPageKeys)
    val autoTarget = mutableStateOf<EditorToolbarPageKey?>(null)
    val autoTargetRevision = mutableStateOf(0L)
    val textScrollState =
      setRetainedDynamicToolbarContent(visible, pageKeys, autoTarget, autoTargetRevision)
    goToTextPageAtEnd(textScrollState)
    val textScroll = textScrollState.value

    visible.value = false
    waitForIdle()

    visible.value = true
    waitForIdle()

    assertEquals(textScroll, textScrollState.value)
    assertPageActive(TextPageTag)
  }

  @Test
  fun manualToolbarHistorySurvivesToolbarRecompose() = runComposeUiTest {
    val visible = mutableStateOf(true)
    val pageKeys =
      mutableStateOf(
        listOf(
          EditorToolbarPageKey.Main,
          EditorToolbarPageKey.Text,
          EditorToolbarPageKey.Blockquote,
        )
      )
    val autoTarget = mutableStateOf<EditorToolbarPageKey?>(null)
    val autoTargetRevision = mutableStateOf(0L)
    val textScrollState =
      setRetainedDynamicToolbarContent(visible, pageKeys, autoTarget, autoTargetRevision)
    goToTextPage()
    goToBlockquotePage(textScrollState)

    visible.value = false
    waitForIdle()
    pageKeys.value = listOf(EditorToolbarPageKey.Main, EditorToolbarPageKey.Text)
    visible.value = true
    autoTargetRevision.value++
    waitForIdle()

    assertPageActive(TextPageTag)
  }

  @Test
  fun indicatorPulseIncrementsWhenToolbarComposesAgain() = runComposeUiTest {
    val visible = mutableStateOf(true)
    lateinit var pagerState: ToolbarPagerState
    setContent {
      val textScrollState = rememberScrollState()
      pagerState = rememberToolbarPagerState()
      ToolbarTestContent(
        textScrollState = textScrollState,
        visible = visible.value,
        pagerState = pagerState,
      )
    }
    waitForIdle()
    val firstPulse = pagerState.indicatorPulse
    assertTrue(firstPulse > 0, "indicator should pulse when the toolbar first appears")

    visible.value = false
    waitForIdle()
    visible.value = true
    waitForIdle()

    assertTrue(
      pagerState.indicatorPulse > firstPulse,
      "indicator should pulse when the toolbar appears again",
    )
  }

  @Test
  fun horizontalSwipeFromBottomGapMovesToolbarPager() = runComposeUiTest {
    setToolbarContent(includeBottomGapInTouchArea = true)

    onNodeWithTag(ToolbarTag).performTouchInput {
      val start = Offset(x = width * 0.5f, y = height - ToolbarBottomPadding.toPx() / 2f)
      swipe(start = start, end = start - Offset(x = width * 0.7f, y = 0f), durationMillis = 120)
    }
    waitForIdle()

    assertPageActive(TextPageTag)
  }

  @Test
  fun verticalSwipesFromBottomGapStayWithToolbar() = runComposeUiTest {
    lateinit var pagerState: ToolbarPagerState
    var toolbarDismissals = 0
    setContent {
      pagerState = rememberToolbarPagerState()
      ToolbarTestContent(
        textScrollState = rememberScrollState(),
        pagerState = pagerState,
        onToolbarDismissRequest = { toolbarDismissals++ },
        includeBottomGapInTouchArea = true,
      )
    }
    waitForIdle()
    val pulseBeforeSwipe = pagerState.indicatorPulse

    swipeToolbarVerticallyFromBottomGap(up = true)

    assertTrue(pagerState.indicatorPulse > pulseBeforeSwipe)

    swipeToolbarVerticallyFromBottomGap(up = false)

    assertEquals(1, toolbarDismissals)
  }

  @Test
  fun tapInBottomGapDoesNotClickToolbarButtons() = runComposeUiTest {
    var fixedActionClicks = 0
    var toolbarButtonClicks = 0
    setToolbarContent(
      onKeyboardDismissRequest = { fixedActionClicks++ },
      onMainButtonClick = { toolbarButtonClicks++ },
      includeBottomGapInTouchArea = true,
    )

    onNodeWithTag(ToolbarTag).performTouchInput {
      val gapY = height - ToolbarBottomPadding.toPx() + 1.dp.toPx()
      click(Offset(x = width * 0.1f, y = gapY))
      click(Offset(x = width * 0.9f, y = gapY))
    }
    waitForIdle()

    assertEquals(0, toolbarButtonClicks)
    assertEquals(0, fixedActionClicks)
  }

  @Test
  fun bottomGapDoesNotMoveVisibleToolbars() = runComposeUiTest {
    setContent {
      ToolbarTestContent(textScrollState = rememberScrollState(), secondaryToolbarVisible = true)
    }
    waitForIdle()
    val originalMainBounds = onNodeWithTag(MainPageTag).fetchSemanticsNode().boundsInRoot
    val originalSecondaryBounds =
      onNodeWithTag(SecondaryToolbarTag).fetchSemanticsNode().boundsInRoot

    setContent {
      ToolbarTestContent(
        textScrollState = rememberScrollState(),
        secondaryToolbarVisible = true,
        includeBottomGapInTouchArea = true,
      )
    }
    waitForIdle()
    val expandedMainBounds = onNodeWithTag(MainPageTag).fetchSemanticsNode().boundsInRoot
    val expandedSecondaryBounds =
      onNodeWithTag(SecondaryToolbarTag).fetchSemanticsNode().boundsInRoot

    assertNear(
      originalMainBounds.top,
      expandedMainBounds.top,
      "bottom gap should not move the main toolbar top",
    )
    assertNear(
      originalMainBounds.bottom,
      expandedMainBounds.bottom,
      "bottom gap should not move the main toolbar bottom",
    )
    assertNear(
      originalSecondaryBounds.top,
      expandedSecondaryBounds.top,
      "bottom gap should not move the secondary toolbar top",
    )
    assertNear(
      originalSecondaryBounds.bottom,
      expandedSecondaryBounds.bottom,
      "bottom gap should not move the secondary toolbar bottom",
    )
  }

  @Test
  fun swipeUpFromFixedActionRevealsIndicatorAndRestartsTimeoutWithoutClickingButton() =
    runComposeUiTest {
      lateinit var pagerState: ToolbarPagerState
      var fixedActionClicks = 0
      mainClock.autoAdvance = false
      setContent {
        pagerState = rememberToolbarPagerState()
        ToolbarTestContent(
          textScrollState = rememberScrollState(),
          pagerState = pagerState,
          onToolbarDismissRequest = {},
          onKeyboardDismissRequest = { fixedActionClicks++ },
        )
      }
      mainClock.advanceTimeByFrame()
      mainClock.advanceTimeBy(ToolbarIndicatorVisibleMillis + 1)
      mainClock.advanceTimeByFrame()
      runOnIdle { assertFalse(pagerState.indicatorVisible) }

      swipeToolbarVerticallyFromFixedAction(up = true)
      mainClock.advanceTimeByFrame()

      runOnIdle {
        assertTrue(pagerState.indicatorVisible)
        assertEquals(0, fixedActionClicks)
        assertEquals(EditorToolbarPageKey.Main, pagerState.settledPageKey)
      }

      mainClock.advanceTimeBy(ToolbarIndicatorVisibleMillis / 2)
      swipeToolbarVerticallyFromFixedAction(up = true)
      mainClock.advanceTimeByFrame()
      mainClock.advanceTimeBy(ToolbarIndicatorVisibleMillis - 100)

      runOnIdle { assertTrue(pagerState.indicatorVisible) }

      mainClock.advanceTimeBy(101)
      mainClock.advanceTimeByFrame()
      runOnIdle { assertFalse(pagerState.indicatorVisible) }
    }

  @Test
  fun swipeDownFromFixedActionDismissesToolbarOnceWithoutClickingButton() = runComposeUiTest {
    var toolbarDismissals = 0
    var fixedActionClicks = 0
    setToolbarContent(
      onToolbarDismissRequest = { toolbarDismissals++ },
      onKeyboardDismissRequest = { fixedActionClicks++ },
    )

    swipeToolbarVerticallyFromFixedAction(up = false)

    assertEquals(1, toolbarDismissals)
    assertEquals(0, fixedActionClicks)
  }

  @Test
  fun horizontalSwipeFromFixedActionDoesNotMoveToolbarPagerOrClickButton() = runComposeUiTest {
    var fixedActionClicks = 0
    setToolbarContent(onKeyboardDismissRequest = { fixedActionClicks++ })

    onNodeWithTag(ToolbarTag).performTouchInput {
      val start =
        Offset(
          x = width - ToolbarFixedActionWidth.toPx() / 2f,
          y = height - ToolbarHeight.toPx() / 2f,
        )
      swipe(start = start, end = start - Offset(x = width * 0.7f, y = 0f), durationMillis = 120)
    }
    waitForIdle()

    assertPageActive(MainPageTag)
    assertEquals(0, fixedActionClicks)
  }

  @Test
  fun swipeDownBelowActivationThresholdDoesNotDismissToolbar() = runComposeUiTest {
    var toolbarDismissals = 0
    setToolbarContent(onToolbarDismissRequest = { toolbarDismissals++ })

    swipeToolbarVerticallyFromFixedAction(up = false, downDistance = 35.dp)

    assertEquals(0, toolbarDismissals)
  }

  @Test
  fun swipeDownAboveActivationThresholdDismissesOnlyOnRelease() = runComposeUiTest {
    var toolbarDismissals = 0
    setToolbarContent(onToolbarDismissRequest = { toolbarDismissals++ })

    startToolbarVerticalDrag(37.dp)

    assertEquals(0, toolbarDismissals)

    releaseToolbarVerticalDrag()

    assertEquals(1, toolbarDismissals)
  }

  @Test
  fun swipeDownUsesHysteresisForReleaseDecision() = runComposeUiTest {
    var toolbarDismissals = 0
    setToolbarContent(onToolbarDismissRequest = { toolbarDismissals++ })

    startToolbarVerticalDrag(37.dp)
    moveToolbarVerticalDragTo(33.dp)
    releaseToolbarVerticalDrag()

    assertEquals(1, toolbarDismissals, "33dp should remain armed after crossing 36dp")

    startToolbarVerticalDrag(37.dp)
    moveToolbarVerticalDragTo(32.dp)
    releaseToolbarVerticalDrag()

    assertEquals(1, toolbarDismissals, "32dp should disarm before release")
  }

  @Test
  fun swipeDownHapticRepeatsAfterDisarmingAndRearming() = runComposeUiTest {
    val haptics = mutableListOf<HapticFeedbackType>()
    val hapticFeedback =
      object : HapticFeedback {
        override fun performHapticFeedback(hapticFeedbackType: HapticFeedbackType) {
          haptics += hapticFeedbackType
        }
      }
    setContent {
      ToolbarTestContent(textScrollState = rememberScrollState(), hapticFeedback = hapticFeedback)
    }
    waitForIdle()

    startToolbarVerticalDrag(37.dp)
    moveToolbarVerticalDragTo(33.dp)
    moveToolbarVerticalDragTo(32.dp)
    moveToolbarVerticalDragTo(37.dp)
    releaseToolbarVerticalDrag()

    assertEquals(
      listOf(
        HapticFeedbackType.GestureThresholdActivate,
        HapticFeedbackType.GestureThresholdActivate,
      ),
      haptics,
    )
  }

  @Test
  fun dismissSwipeVisualFeedbackUsesStableOriginAcrossThreshold() = runComposeUiTest {
    mainClock.autoAdvance = false
    var density = 0f
    setContent {
      density = LocalDensity.current.density
      ToolbarTestContent(textScrollState = rememberScrollState())
    }
    mainClock.advanceTimeByFrame()
    val initialTop = pageTop(MainPageTag)

    startToolbarVerticalDrag(20.dp)
    mainClock.advanceTimeByFrame()
    assertNear(initialTop + 4f * density, pageTop(MainPageTag), "unarmed drag should follow 20%")

    moveToolbarVerticalDragTo(37.dp)
    mainClock.advanceTimeBy(500)
    assertNear(initialTop + 16f * density, pageTop(MainPageTag), "armed drag should shift 16dp")

    moveToolbarVerticalDragTo(33.dp)
    mainClock.advanceTimeByFrame()
    assertNear(initialTop + 16f * density, pageTop(MainPageTag), "33dp should remain armed")

    moveToolbarVerticalDragTo(32.dp)
    mainClock.advanceTimeBy(500)
    assertNear(initialTop + 6.4f * density, pageTop(MainPageTag), "32dp should return to 20%")

    releaseToolbarVerticalDrag()
    mainClock.autoAdvance = true
    waitForIdle()
  }

  @Test
  fun primaryDismissDragMovesVisibleToolbarStackTogether() = runComposeUiTest {
    mainClock.autoAdvance = false
    var density = 0f
    setContent {
      density = LocalDensity.current.density
      ToolbarTestContent(textScrollState = rememberScrollState(), secondaryToolbarVisible = true)
    }
    mainClock.advanceTimeByFrame()
    val initialPrimaryTop = pageTop(MainPageTag)
    val initialSecondaryTop = pageTop(SecondaryToolbarTag)
    val initialIndicatorTop = indicatorTop()

    startToolbarVerticalDrag(37.dp)
    mainClock.advanceTimeBy(500)

    val expectedOffset = 16f * density
    assertNear(
      initialPrimaryTop + expectedOffset,
      pageTop(MainPageTag),
      "primary drag should move the primary toolbar",
    )
    assertNear(
      initialSecondaryTop + expectedOffset,
      pageTop(SecondaryToolbarTag),
      "primary drag should move the secondary toolbar",
    )
    assertNear(
      initialIndicatorTop + expectedOffset,
      indicatorTop(),
      "primary drag should move the indicator",
    )

    releaseToolbarVerticalDrag()
    mainClock.autoAdvance = true
    waitForIdle()
  }

  @Test
  fun secondaryContentKeepsFinalHeightWhileStackSlotExpands() = runComposeUiTest {
    mainClock.autoAdvance = false
    lateinit var secondaryToolbarVisible: MutableState<Boolean>
    var density = 0f
    setContent {
      secondaryToolbarVisible = remember { mutableStateOf(false) }
      density = LocalDensity.current.density
      ToolbarTestContent(
        textScrollState = rememberScrollState(),
        secondaryToolbarVisible = secondaryToolbarVisible.value,
      )
    }
    mainClock.advanceTimeByFrame()

    runOnIdle { secondaryToolbarVisible.value = true }
    mainClock.advanceTimeByFrame()
    mainClock.advanceTimeBy(ToolbarSecondaryVisibilityMillis.toLong() / 2)

    val secondaryHeight =
      onNodeWithTag(SecondaryToolbarTag).fetchSemanticsNode().boundsInRoot.height
    assertNear(
      ToolbarSecondaryHeight.value * density,
      secondaryHeight,
      "secondary content should be measured at its final height during stack expansion",
    )

    mainClock.autoAdvance = true
    waitForIdle()
  }

  @Test
  fun secondaryExitMovesIndicatorDuringTransition() = runComposeUiTest {
    mainClock.autoAdvance = false
    lateinit var secondaryToolbarVisible: MutableState<Boolean>
    var density = 0f
    setContent {
      secondaryToolbarVisible = remember { mutableStateOf(true) }
      density = LocalDensity.current.density
      ToolbarTestContent(
        textScrollState = rememberScrollState(),
        secondaryToolbarVisible = secondaryToolbarVisible.value,
        toolbarLayoutHeightOverride = ToolbarStackHeight + ToolbarSecondaryStackHeight,
      )
    }
    mainClock.advanceTimeBy(ToolbarSecondaryVisibilityMillis.toLong() * 2)
    val initialIndicatorTop = indicatorTop()

    runOnIdle { secondaryToolbarVisible.value = false }
    mainClock.advanceTimeByFrame()
    mainClock.advanceTimeBy(ToolbarSecondaryVisibilityMillis.toLong() / 2)

    val indicatorTopDuringExit = indicatorTop()
    val finalIndicatorTop = initialIndicatorTop + ToolbarSecondaryStackHeight.value * density
    assertTrue(
      indicatorTopDuringExit > initialIndicatorTop + 2f,
      "indicator should start moving down while the secondary toolbar exits",
    )
    assertTrue(
      indicatorTopDuringExit < finalIndicatorTop - 2f,
      "indicator should still be moving during the secondary toolbar exit",
    )

    mainClock.autoAdvance = true
    waitForIdle()
  }

  @Test
  fun secondaryDismissDragDoesNotMoveToolbarStack() = runComposeUiTest {
    mainClock.autoAdvance = false
    setContent {
      ToolbarTestContent(textScrollState = rememberScrollState(), secondaryToolbarVisible = true)
    }
    mainClock.advanceTimeByFrame()
    val initialPrimaryTop = pageTop(MainPageTag)
    val initialSecondaryTop = pageTop(SecondaryToolbarTag)
    val initialIndicatorTop = indicatorTop()

    startSecondaryToolbarVerticalDrag(37.dp)
    mainClock.advanceTimeBy(500)

    assertNear(
      initialPrimaryTop,
      pageTop(MainPageTag),
      "secondary drag should not move the primary toolbar",
    )
    assertNear(
      initialSecondaryTop,
      pageTop(SecondaryToolbarTag),
      "secondary drag should not move the secondary toolbar",
    )
    assertNear(initialIndicatorTop, indicatorTop(), "secondary drag should not move the indicator")

    releaseSecondaryToolbarVerticalDrag()
    mainClock.autoAdvance = true
    waitForIdle()
  }

  @Test
  fun secondarySwipeDownClearsOnlySecondaryToolbar() = runComposeUiTest {
    var secondaryClears = 0
    var toolbarDismissals = 0
    var keyboardDismissals = 0
    val haptics = mutableListOf<HapticFeedbackType>()
    val hapticFeedback =
      object : HapticFeedback {
        override fun performHapticFeedback(hapticFeedbackType: HapticFeedbackType) {
          haptics += hapticFeedbackType
        }
      }
    setContent {
      ToolbarTestContent(
        textScrollState = rememberScrollState(),
        secondaryToolbarVisible = true,
        onSecondaryToolbarClear = { secondaryClears++ },
        onToolbarDismissRequest = { toolbarDismissals++ },
        onKeyboardDismissRequest = { keyboardDismissals++ },
        hapticFeedback = hapticFeedback,
      )
    }
    waitForIdle()

    swipeSecondaryToolbarDown(40.dp)

    assertEquals(1, secondaryClears)
    assertEquals(0, toolbarDismissals)
    assertEquals(0, keyboardDismissals)
    assertEquals(emptyList(), haptics)
    assertNear(0f, pageLeft(MainPageTag), "primary toolbar should remain visible")
  }

  @Test
  fun secondarySwipeDownBelowActivationThresholdDoesNotClearSecondaryToolbar() = runComposeUiTest {
    var secondaryClears = 0
    var toolbarDismissals = 0
    setContent {
      ToolbarTestContent(
        textScrollState = rememberScrollState(),
        secondaryToolbarVisible = true,
        onSecondaryToolbarClear = { secondaryClears++ },
        onToolbarDismissRequest = { toolbarDismissals++ },
      )
    }
    waitForIdle()

    swipeSecondaryToolbarDown(35.dp)

    assertEquals(0, secondaryClears)
    assertEquals(0, toolbarDismissals)
  }

  @Test
  fun swipeUpFromToolbarButtonRevealsIndicatorWithoutClickingButton() = runComposeUiTest {
    lateinit var pagerState: ToolbarPagerState
    var toolbarButtonClicks = 0
    setContent {
      pagerState = rememberToolbarPagerState()
      ToolbarTestContent(
        textScrollState = rememberScrollState(),
        pagerState = pagerState,
        onToolbarDismissRequest = {},
        onMainButtonClick = { toolbarButtonClicks++ },
      )
    }
    waitForIdle()
    val pulseBeforeSwipe = pagerState.indicatorPulse

    swipeToolbarVerticallyFromMainButton(up = true)

    assertTrue(pagerState.indicatorPulse > pulseBeforeSwipe)
    assertEquals(0, toolbarButtonClicks)
    assertPageActive(MainPageTag)
  }

  @Test
  fun swipeDownFromToolbarButtonDismissesToolbarOnceWithoutClickingButton() = runComposeUiTest {
    var toolbarDismissals = 0
    var toolbarButtonClicks = 0
    setToolbarContent(
      onToolbarDismissRequest = { toolbarDismissals++ },
      onMainButtonClick = { toolbarButtonClicks++ },
    )

    swipeToolbarVerticallyFromMainButton(up = false)

    assertEquals(1, toolbarDismissals)
    assertEquals(0, toolbarButtonClicks)
  }

  @Test
  fun toolbarButtonsStillClickWithoutVerticalSwipe() = runComposeUiTest {
    var fixedActionClicks = 0
    var toolbarButtonClicks = 0
    setToolbarContent(
      onToolbarDismissRequest = {},
      onKeyboardDismissRequest = { fixedActionClicks++ },
      onMainButtonClick = { toolbarButtonClicks++ },
    )

    onNodeWithTag(MainButtonTag).performTouchInput { click(center) }
    onNodeWithContentDescription("에디터 포커스 해제").performTouchInput { click(center) }
    waitForIdle()

    assertEquals(1, toolbarButtonClicks)
    assertEquals(1, fixedActionClicks)
  }

  @Test
  fun horizontalToolbarSwipeDoesNotDismissToolbar() = runComposeUiTest {
    var toolbarDismissals = 0
    setToolbarContent(onToolbarDismissRequest = { toolbarDismissals++ })

    swipeToolbarLeft(distanceFraction = 0.7f)

    assertEquals(0, toolbarDismissals)
    assertPageActive(TextPageTag)
  }

  @Test
  fun swipeUpWithOnePageDoesNotPulseIndicator() = runComposeUiTest {
    lateinit var pagerState: ToolbarPagerState
    setContent {
      pagerState = rememberToolbarPagerState()
      ToolbarTestContent(
        textScrollState = rememberScrollState(),
        pageKeys = listOf(EditorToolbarPageKey.Main),
        pagerState = pagerState,
        onToolbarDismissRequest = {},
      )
    }
    waitForIdle()
    val pulseBeforeSwipe = pagerState.indicatorPulse

    swipeToolbarVerticallyFromFixedAction(up = true)

    assertEquals(pulseBeforeSwipe, pagerState.indicatorPulse)
  }

  @Test
  fun ancestorConsumedVerticalMovementDoesNotTriggerToolbarSwipe() = runComposeUiTest {
    lateinit var pagerState: ToolbarPagerState
    var toolbarDismissals = 0
    val consumingAncestor =
      Modifier.pointerInput(Unit) {
        awaitPointerEventScope {
          while (true) {
            val event = awaitPointerEvent(PointerEventPass.Initial)
            event.changes.forEach { change ->
              if (change.pressed && change.previousPressed) {
                change.consume()
              }
            }
          }
        }
      }
    setContent {
      pagerState = rememberToolbarPagerState()
      ToolbarTestContent(
        textScrollState = rememberScrollState(),
        pagerState = pagerState,
        onToolbarDismissRequest = { toolbarDismissals++ },
        ancestorModifier = consumingAncestor,
      )
    }
    waitForIdle()
    val pulseBeforeSwipe = pagerState.indicatorPulse

    swipeToolbarVerticallyFromFixedAction(up = true)
    swipeToolbarVerticallyFromFixedAction(up = false)

    assertEquals(pulseBeforeSwipe, pagerState.indicatorPulse)
    assertEquals(0, toolbarDismissals)
  }

  @Test
  fun manualToolbarHistoryKeepsMostRecentUniquePageKeys() {
    val pagerState = ToolbarPagerState()

    pagerState.recordManualPageKey(EditorToolbarPageKey.Text)
    pagerState.recordManualPageKey(EditorToolbarPageKey.Image)
    pagerState.recordManualPageKey(EditorToolbarPageKey.Text)

    assertEquals(
      listOf(EditorToolbarPageKey.Text, EditorToolbarPageKey.Image, EditorToolbarPageKey.Main),
      pagerState.recentManualPageKeys,
    )
  }

  private fun ComposeUiTest.setToolbarContent(
    onToolbarDismissRequest: () -> Unit = {},
    onKeyboardDismissRequest: () -> Unit = {},
    onMainButtonClick: () -> Unit = {},
    includeBottomGapInTouchArea: Boolean = false,
  ): ScrollState {
    lateinit var textScrollState: ScrollState
    setContent {
      textScrollState = rememberScrollState()
      ToolbarTestContent(
        textScrollState = textScrollState,
        onToolbarDismissRequest = onToolbarDismissRequest,
        onKeyboardDismissRequest = onKeyboardDismissRequest,
        onMainButtonClick = onMainButtonClick,
        includeBottomGapInTouchArea = includeBottomGapInTouchArea,
      )
    }
    waitForIdle()
    assertPageActive(MainPageTag)
    return textScrollState
  }

  private fun ComposeUiTest.setRetainedDynamicToolbarContent(
    visible: MutableState<Boolean>,
    pageKeys: MutableState<List<EditorToolbarPageKey>>,
    autoTarget: MutableState<EditorToolbarPageKey?>,
    autoTargetRevision: MutableState<Long>,
  ): ScrollState {
    lateinit var textScrollState: ScrollState
    setContent {
      textScrollState = rememberScrollState()
      val pagerState = rememberToolbarPagerState()
      ToolbarTestContent(
        textScrollState = textScrollState,
        pageKeys = pageKeys.value,
        autoTargetPageKey = autoTarget.value,
        autoTargetRevision = autoTargetRevision.value,
        visible = visible.value,
        pagerState = pagerState,
      )
    }
    waitForIdle()
    assertPageActive(MainPageTag)
    return textScrollState
  }

  private fun ComposeUiTest.setDynamicToolbarContent(
    pageKeys: MutableState<List<EditorToolbarPageKey>>,
    autoTarget: MutableState<EditorToolbarPageKey?>,
    autoTargetRevision: MutableState<Long>,
  ): ScrollState {
    lateinit var textScrollState: ScrollState
    setContent {
      textScrollState = rememberScrollState()
      ToolbarTestContent(
        textScrollState = textScrollState,
        pageKeys = pageKeys.value,
        autoTargetPageKey = autoTarget.value,
        autoTargetRevision = autoTargetRevision.value,
      )
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

  private fun ComposeUiTest.goToBlockquotePage(textScrollState: ScrollState) {
    swipeToolbarLeft(distanceFraction = 4.9f, durationMillis = 500)
    assertEquals(textScrollState.maxValue, textScrollState.value)
    assertPageActive(TextPageTag)
    swipeToolbarLeft(distanceFraction = 0.82f)
    assertPageActive(BlockquotePageTag)
  }

  private fun ComposeUiTest.goToFoldPage(textScrollState: ScrollState) {
    swipeToolbarLeft(distanceFraction = 4.9f, durationMillis = 500)
    assertEquals(textScrollState.maxValue, textScrollState.value)
    assertPageActive(TextPageTag)
    swipeToolbarLeft(distanceFraction = 0.82f)
    assertPageActive(FoldPageTag)
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

  private fun ComposeUiTest.swipeToolbarRightWithoutIdle(
    distanceFraction: Float,
    durationMillis: Long = 120,
  ) {
    onNodeWithTag(ToolbarTag).performTouchInput {
      swipe(
        start = Offset(x = width * 0.18f, y = height - 16f),
        end = Offset(x = width * (0.18f + distanceFraction), y = height - 16f),
        durationMillis = durationMillis,
      )
    }
  }

  private fun ComposeUiTest.startToolbarDrag(startFraction: Float, endFraction: Float) {
    onNodeWithTag(ToolbarTag).performTouchInput {
      down(Offset(x = width * startFraction, y = height - 16f))
      moveTo(Offset(x = width * endFraction, y = height - 16f))
    }
    waitForIdle()
  }

  private fun ComposeUiTest.releaseToolbarDrag() {
    onNodeWithTag(ToolbarTag).performTouchInput { up() }
    waitForIdle()
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

  private fun ComposeUiTest.swipeToolbarVerticallyFromFixedAction(
    up: Boolean,
    downDistance: Dp = 40.dp,
  ) {
    onNodeWithTag(ToolbarTag).performTouchInput {
      val x = width - ToolbarFixedActionWidth.toPx() / 2f
      val startY = height - ToolbarHeight.toPx() / 2f
      val distance = (if (up) 20.dp else downDistance).toPx()
      swipe(
        start = Offset(x = x, y = startY),
        end = Offset(x = x, y = startY + if (up) -distance else distance),
        durationMillis = 120,
      )
    }
    waitForIdle()
  }

  private fun ComposeUiTest.swipeToolbarVerticallyFromMainButton(up: Boolean) {
    onNodeWithTag(MainButtonTag).performTouchInput {
      val distance = (if (up) 20.dp else 40.dp).toPx()
      swipe(
        start = center,
        end = center + Offset(x = 0f, y = if (up) -distance else distance),
        durationMillis = 120,
      )
    }
    waitForIdle()
  }

  private fun ComposeUiTest.swipeToolbarVerticallyFromBottomGap(up: Boolean) {
    onNodeWithTag(ToolbarTag).performTouchInput {
      val start = Offset(x = width * 0.5f, y = height - ToolbarBottomPadding.toPx() / 2f)
      val distance = (if (up) 20.dp else 40.dp).toPx()
      swipe(
        start = start,
        end = start + Offset(x = 0f, y = if (up) -distance else distance),
        durationMillis = 120,
      )
    }
    waitForIdle()
  }

  private fun ComposeUiTest.startToolbarVerticalDrag(distance: Dp) {
    onNodeWithTag(ToolbarTag).performTouchInput {
      val x = width - ToolbarFixedActionWidth.toPx() / 2f
      val startY = height - ToolbarHeight.toPx() / 2f
      down(Offset(x = x, y = startY))
      moveTo(Offset(x = x, y = startY + distance.toPx()))
    }
    waitForIdle()
  }

  private fun ComposeUiTest.moveToolbarVerticalDragTo(distance: Dp) {
    onNodeWithTag(ToolbarTag).performTouchInput {
      val x = width - ToolbarFixedActionWidth.toPx() / 2f
      val startY = height - ToolbarHeight.toPx() / 2f
      moveTo(Offset(x = x, y = startY + distance.toPx()))
    }
    waitForIdle()
  }

  private fun ComposeUiTest.releaseToolbarVerticalDrag() {
    onNodeWithTag(ToolbarTag).performTouchInput { up() }
    waitForIdle()
  }

  private fun ComposeUiTest.startSecondaryToolbarVerticalDrag(distance: Dp) {
    onNodeWithTag(SecondaryToolbarTag).performTouchInput {
      down(center)
      moveTo(center + Offset(x = 0f, y = distance.toPx()))
    }
    waitForIdle()
  }

  private fun ComposeUiTest.releaseSecondaryToolbarVerticalDrag() {
    onNodeWithTag(SecondaryToolbarTag).performTouchInput { up() }
    waitForIdle()
  }

  private fun ComposeUiTest.swipeSecondaryToolbarDown(distance: Dp) {
    onNodeWithTag(SecondaryToolbarTag).performTouchInput {
      swipe(
        start = center,
        end = center + Offset(x = 0f, y = distance.toPx()),
        durationMillis = 120,
      )
    }
    waitForIdle()
  }

  private fun ComposeUiTest.tapToolbarIndicatorPage(pageIndex: Int, pageCount: Int) {
    onNodeWithTag(ToolbarTag).performTouchInput {
      val itemPx = ToolbarIndicatorItemSize.toPx()
      val gapPx = ToolbarIndicatorItemGap.toPx()
      val paddingPx = ToolbarIndicatorPadding.toPx()
      val indicatorWidth = paddingPx * 2 + itemPx * pageCount + gapPx * (pageCount - 1)
      val indicatorLeft = (width - indicatorWidth) / 2f
      val x = indicatorLeft + paddingPx + itemPx / 2f + (itemPx + gapPx) * pageIndex
      click(Offset(x = x, y = ToolbarIndicatorHeight.toPx() / 2f))
    }
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

  private fun SemanticsNodeInteractionsProvider.pageRight(tag: String): Float =
    onNodeWithTag(tag).fetchSemanticsNode().boundsInRoot.right

  private fun SemanticsNodeInteractionsProvider.pageTop(tag: String): Float =
    onNodeWithTag(tag).fetchSemanticsNode().boundsInRoot.top

  private fun SemanticsNodeInteractionsProvider.indicatorTop(): Float =
    onNodeWithContentDescription("메인 툴바").fetchSemanticsNode().boundsInRoot.top

  private fun assertNear(expected: Float, actual: Float, message: String) {
    assertTrue(abs(expected - actual) <= 2f, "$message. expected=$expected actual=$actual")
  }

  @Composable
  private fun ToolbarTestContent(
    textScrollState: ScrollState,
    pageKeys: List<EditorToolbarPageKey> = DefaultPageKeys,
    autoTargetPageKey: EditorToolbarPageKey? = null,
    autoTargetRevision: Long = 0L,
    visible: Boolean = true,
    pagerState: ToolbarPagerState = rememberToolbarPagerState(),
    textItemWidth: Dp = 80.dp,
    onToolbarDismissRequest: () -> Unit = {},
    onKeyboardDismissRequest: () -> Unit = {},
    onSecondaryToolbarClear: () -> Unit = {},
    onMainButtonClick: () -> Unit = {},
    ancestorModifier: Modifier = Modifier,
    includeBottomGapInTouchArea: Boolean = false,
    secondaryToolbarVisible: Boolean = false,
    toolbarLayoutHeightOverride: Dp? = null,
    hapticFeedback: HapticFeedback? = null,
  ) {
    val pages =
      rememberToolbarTestPages(
        textScrollState = textScrollState,
        pageKeys = pageKeys,
        textItemWidth = textItemWidth,
        onMainButtonClick = onMainButtonClick,
      )
    val commandScope = rememberCoroutineScope()
    val bottomTouchGapHeight = if (includeBottomGapInTouchArea) ToolbarBottomPadding else 0.dp
    val toolbarLayoutHeight =
      toolbarLayoutHeightOverride
        ?: (ToolbarStackHeight +
          bottomTouchGapHeight +
          if (secondaryToolbarVisible) ToolbarSecondaryStackHeight else 0.dp)
    ToolbarTestTheme(hapticFeedback = hapticFeedback) {
      Box(ancestorModifier) {
        Box(Modifier.width(360.dp).height(toolbarLayoutHeight).testTag(ToolbarTag)) {
          if (visible) {
            EditorToolbarPages(
              pages = pages,
              commandScope = commandScope,
              pagerState = pagerState,
              autoTargetPageKey = autoTargetPageKey,
              autoTargetKey = autoTargetRevision,
              editorFocused = true,
              activeBottomPanel = null,
              fixedAction = ToolbarFixedAction.DismissInput,
              onEditorInputRequest = {},
              onKeyboardDismissRequest = onKeyboardDismissRequest,
              onToolbarDismissRequest = onToolbarDismissRequest,
              onBottomPanelToggle = { _, _ -> },
              onSecondaryToolbarClear = onSecondaryToolbarClear,
              secondaryToolbarVisible = secondaryToolbarVisible,
              secondaryToolbar = {
                Box(
                  Modifier.fillMaxWidth()
                    .height(ToolbarSecondaryHeight)
                    .testTag(SecondaryToolbarTag)
                )
              },
              includeBottomGapInTouchArea = includeBottomGapInTouchArea,
              modifier = Modifier.fillMaxSize(),
            )
          }
        }
      }
    }
  }

  @Composable
  private fun ToolbarTestTheme(
    hapticFeedback: HapticFeedback? = null,
    content: @Composable () -> Unit,
  ) {
    CompositionLocalProvider(
      LocalAppColors provides LightColors,
      LocalAppShadows provides LightAppShadows,
      LocalThemeMode provides ResolvedThemeMode.Light,
      LocalHazeBlurStyle provides
        HazeBlurStyle(blurRadius = 20.dp, noiseFactor = 0f, colorEffects = listOf()),
    ) {
      if (hapticFeedback == null) {
        content()
      } else {
        CompositionLocalProvider(LocalHapticFeedback provides hapticFeedback, content = content)
      }
    }
  }

  @Composable
  private fun rememberToolbarTestPages(
    textScrollState: ScrollState,
    pageKeys: List<EditorToolbarPageKey>,
    textItemWidth: Dp,
    onMainButtonClick: () -> Unit,
  ): List<EditorToolbarPage> =
    remember(textScrollState, pageKeys, textItemWidth, onMainButtonClick) {
      pageKeys.map { key ->
        when (key) {
          EditorToolbarPageKey.Main ->
            EditorToolbarPage(
              key = EditorToolbarPageKey.Main,
              icon = Lucide.CircleSmall,
              contentDescription = "메인 툴바",
              content = {
                Box(Modifier.fillMaxSize().testTag(MainPageTag)) {
                  EditorToolbarButton(
                    icon = Lucide.CircleSmall,
                    contentDescription = "테스트 툴바 버튼",
                    onClick = onMainButtonClick,
                    modifier = Modifier.align(Alignment.CenterStart).testTag(MainButtonTag),
                  )
                }
              },
            )
          EditorToolbarPageKey.Text ->
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
                      Modifier.size(width = textItemWidth, height = ToolbarButtonSize)
                        .testTag("text-item-$index")
                    )
                  }
                  if (scope.hasNextPage) {
                    EditorToolbarPageIndicator()
                  }
                }
              },
            )
          EditorToolbarPageKey.Image ->
            EditorToolbarPage(
              key = EditorToolbarPageKey.Image,
              icon = Lucide.Image,
              contentDescription = "이미지 툴바",
              content = { Box(Modifier.fillMaxSize().testTag(ImagePageTag)) },
            )
          EditorToolbarPageKey.File ->
            EditorToolbarPage(
              key = EditorToolbarPageKey.File,
              icon = Lucide.Paperclip,
              contentDescription = "파일 툴바",
              content = { Box(Modifier.fillMaxSize().testTag(FilePageTag)) },
            )
          EditorToolbarPageKey.Blockquote ->
            EditorToolbarPage(
              key = EditorToolbarPageKey.Blockquote,
              icon = Lucide.Quote,
              contentDescription = "인용구 툴바",
              content = { Box(Modifier.fillMaxSize().testTag(BlockquotePageTag)) },
            )
          EditorToolbarPageKey.Fold ->
            EditorToolbarPage(
              key = EditorToolbarPageKey.Fold,
              icon = Lucide.TextSelect,
              contentDescription = "접기 툴바",
              content = { Box(Modifier.fillMaxSize().testTag(FoldPageTag)) },
            )
          else ->
            EditorToolbarPage(
              key = key,
              icon = Lucide.CircleSmall,
              contentDescription = key.name,
              content = { Box(Modifier.fillMaxSize()) },
            )
        }
      }
    }

  private companion object {
    const val ToolbarTag = "editor-toolbar"
    const val SecondaryToolbarTag = "secondary-toolbar"
    const val MainButtonTag = "main-toolbar-button"
    const val MainPageTag = "main-page"
    const val TextPageTag = "text-page"
    const val ImagePageTag = "image-page"
    const val FilePageTag = "file-page"
    const val BlockquotePageTag = "blockquote-page"
    const val FoldPageTag = "fold-page"
    val DefaultPageKeys =
      listOf(EditorToolbarPageKey.Main, EditorToolbarPageKey.Text, EditorToolbarPageKey.Image)
  }
}
