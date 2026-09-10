package co.typie.screen.editor.editor.overlay

import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.size
import androidx.compose.runtime.CompositionLocalProvider
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.geometry.Rect
import androidx.compose.ui.geometry.Size
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.toPixelMap
import androidx.compose.ui.input.key.Key
import androidx.compose.ui.platform.LocalDensity
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.semantics.SemanticsProperties
import androidx.compose.ui.test.ExperimentalTestApi
import androidx.compose.ui.test.assertIsDisplayed
import androidx.compose.ui.test.captureToImage
import androidx.compose.ui.test.click
import androidx.compose.ui.test.hasScrollAction
import androidx.compose.ui.test.onAllNodesWithText
import androidx.compose.ui.test.onNodeWithContentDescription
import androidx.compose.ui.test.onNodeWithTag
import androidx.compose.ui.test.onNodeWithText
import androidx.compose.ui.test.onRoot
import androidx.compose.ui.test.performClick
import androidx.compose.ui.test.performKeyInput
import androidx.compose.ui.test.performMouseInput
import androidx.compose.ui.test.performScrollTo
import androidx.compose.ui.test.performTouchInput
import androidx.compose.ui.test.pressKey
import androidx.compose.ui.test.runComposeUiTest
import androidx.compose.ui.test.swipeLeft
import androidx.compose.ui.unit.Density
import androidx.compose.ui.unit.dp
import co.typie.editor.EditorState
import co.typie.editor.PagePoint
import co.typie.editor.ffi.SelectionExpansionUnit
import co.typie.editor.interaction.EditorInteractionGeometry
import co.typie.editor.runtime.EditorContextMenuMode
import co.typie.editor.runtime.EditorContextMenuState
import co.typie.editor.scroll.EditorVisibleArea
import co.typie.ext.clickable
import co.typie.icons.Lucide
import co.typie.screen.editor.editor.toolbar.toolbarIndicatorGestures
import co.typie.ui.component.popover.LocalPopoverOverlayState
import co.typie.ui.component.popover.PopoverMenu
import co.typie.ui.component.popover.PopoverOverlayState
import co.typie.ui.component.sheet.SheetBarButton
import co.typie.ui.input.WindowInputTestHost
import co.typie.ui.theme.AppColors
import co.typie.ui.theme.LightAppShadows
import co.typie.ui.theme.LightColors
import co.typie.ui.theme.LocalAppColors
import co.typie.ui.theme.LocalAppShadows
import co.typie.ui.theme.LocalThemeMode
import co.typie.ui.theme.ResolvedThemeMode
import kotlin.math.roundToInt
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertFalse
import kotlin.test.assertTrue

@OptIn(ExperimentalTestApi::class)
class EditorContextMenuOverlayDesktopTest {
  @Test
  fun onlyExpandedMenusConsumeTheFirstOutsideControlClick() = runComposeUiTest {
    val menu = EditorContextMenuState()
    val geometry =
      object : EditorInteractionGeometry {
        override val density = 1f

        override fun containsDocumentInteraction(positionInRoot: Offset) = false

        override fun resolveInteractionPosition(positionInRoot: Offset): Offset? = null

        override fun isTapEligible(positionInRoot: Offset) = false

        override fun resolvePoint(positionInNode: Offset): PagePoint? = null

        override fun resolvePagePosition(page: Int, x: Float, y: Float): Offset? = null

        override fun resolveEdgeAutoScrollViewport() = null
      }
    var clicks = 0
    var wordSelections = 0
    var selectedPage = 0
    val popover = PopoverOverlayState()
    setContent {
      CompositionLocalProvider(LocalPopoverOverlayState provides popover) {
        WindowInputTestHost(Modifier.size(400.dp, 700.dp)) {
          EditorContextMenuOutsideTapHost(menu, geometry)
          Box(
            Modifier.align(Alignment.TopStart)
              .size(100.dp, 40.dp)
              .testTag("indicator")
              .toolbarIndicatorGestures(
                pageCount = 2,
                currentPageIndex = selectedPage,
                onIndicatorProgress = {},
                onIndicatorDraggingChange = {},
                onPageSelected = { selectedPage = it },
                onInteractionActiveChange = {},
              )
          )
          Box(Modifier.align(Alignment.TopEnd)) {
            PopoverMenu(
              anchor = {
                SheetBarButton(Lucide.ListFilter, "필터", modifier = Modifier.testTag("anchor"))
              }
            ) {
              item(Lucide.Check, "항목") {}
            }
          }
          Box(
            Modifier.align(Alignment.BottomCenter).size(80.dp).testTag("outside").clickable {
              clicks++
            }
          )
          if (menu.visible) {
            EditorSelectionContextMenuOverlay(
              mode = menu.mode,
              onExpandMenu = menu::expand,
              anchor = EditorContextMenuAnchor(200f, 300f, 400f),
              overlaySize = Size(400f, 700f),
              visibleArea = EditorVisibleArea(viewport = Size(400f, 700f)),
              actions =
                EditorContextMenuActions(
                  showCopyCutActions = true,
                  availableExpansionUnits = SelectionExpansionUnit.entries.toSet(),
                  onCopy = {},
                  onCut = {},
                  onPaste = {},
                  onExpandWord = { wordSelections++ },
                  onExpandSentence = {},
                  onExpandParagraph = {},
                  onSelectAll = {},
                  onDismiss = menu::hide,
                ),
              onBoundsInWindowChanged = { menu.boundsInWindow = it },
            )
          }
        }
      }
    }
    runOnIdle {
      menu.show(EditorState.Initial)
      menu.expand()
    }
    onNodeWithText("선택 확장").performMouseInput { click() }
    onNodeWithText("단어 선택").performMouseInput { click() }
    waitForIdle()
    assertEquals(1, wordSelections)
    assertTrue(menu.visible)
    onNodeWithText("선택 확장").performClick()
    onRoot().performKeyInput { pressKey(Key.Escape) }
    waitForIdle()
    assertTrue(menu.visible)
    onNodeWithText("복사").assertIsDisplayed()
    onRoot().performKeyInput { pressKey(Key.Escape) }
    waitForIdle()
    assertFalse(menu.visible, "A second Escape must dismiss the root menu")
    runOnIdle {
      menu.show(EditorState.Initial)
      menu.expand()
    }
    onNodeWithText("선택 확장").performClick()
    val parentBounds = menu.boundsInWindow.first()
    val childBounds = menu.boundsInWindow.last()
    onRoot().performMouseInput { click(Offset(childBounds.left + 1f, parentBounds.top + 8f)) }
    waitForIdle()
    assertFalse(menu.visible, "The gap next to the scaled parent is outside both menu cards")
    for (expandedMenu in listOf(true, false)) {
      repeat(2) { index ->
        val before = clicks
        runOnIdle {
          menu.show(EditorState.Initial)
          if (expandedMenu) menu.expand()
        }
        if (index == 0) onNodeWithTag("outside").performMouseInput { click() }
        else onNodeWithTag("outside").performTouchInput { click() }
        waitForIdle()
        assertFalse(menu.visible)
        assertEquals(before + if (expandedMenu) 0 else 1, clicks)
      }
    }
    val before = clicks
    onNodeWithTag("outside").performMouseInput { click() }
    waitForIdle()
    assertEquals(before + 1, clicks)
    for (expandedMenu in listOf(true, false)) {
      runOnIdle {
        menu.show(EditorState.Initial)
        if (expandedMenu) menu.expand()
      }
      onNodeWithTag("indicator").performMouseInput { click(Offset(width - 5f, center.y)) }
      waitForIdle()
      assertFalse(menu.visible)
      assertEquals(if (expandedMenu) 0 else 1, selectedPage)
    }
    for (expandedMenu in listOf(true, false)) {
      runOnIdle {
        menu.show(EditorState.Initial)
        if (expandedMenu) menu.expand()
      }
      onNodeWithTag("anchor").performMouseInput { click() }
      waitForIdle()
      assertFalse(menu.visible)
      assertEquals(!expandedMenu, popover.acceptsInput)
    }
  }

  @Test
  fun mouseHoldCanScrubIntoTheNewSubmenu() = submenuOpeningPress(mouse = true, outside = false)

  @Test
  fun touchHoldCanScrubIntoTheNewSubmenu() = submenuOpeningPress(mouse = false, outside = false)

  @Test
  fun mouseHoldReleasedOutsideDoesNotSelect() = submenuOpeningPress(mouse = true, outside = true)

  @Test
  fun touchHoldReleasedOutsideDoesNotSelect() = submenuOpeningPress(mouse = false, outside = true)

  private fun submenuOpeningPress(mouse: Boolean, outside: Boolean) = runComposeUiTest {
    mainClock.autoAdvance = false
    var words = 0
    var paragraphs = 0
    setMenuContent(
      mode = EditorContextMenuMode.Expanded,
      onExpandWord = { words++ },
      onExpandParagraph = { paragraphs++ },
    )
    val root = onRoot()
    val trigger = onNodeWithText("선택 확장").fetchSemanticsNode().boundsInRoot.center
    if (mouse)
      root.performMouseInput {
        moveTo(trigger)
        press()
      }
    else root.performTouchInput { down(trigger) }
    mainClock.advanceTimeBy(700)
    waitForIdle()
    for (label in listOf("단어 선택", "문단 선택")) {
      val position = onNodeWithText(label).fetchSemanticsNode().boundsInRoot.center
      if (mouse) root.performMouseInput { moveTo(position) }
      else root.performTouchInput { moveTo(position) }
      mainClock.advanceTimeByFrame()
      waitForIdle()
      assertEquals(0, words + paragraphs, "Scrubbing must wait for release")
    }
    if (outside) {
      if (mouse) root.performMouseInput { moveTo(Offset(1f, 1f)) }
      else root.performTouchInput { moveTo(Offset(1f, 1f)) }
      mainClock.advanceTimeByFrame()
    }
    if (mouse) root.performMouseInput { release() } else root.performTouchInput { up() }
    mainClock.advanceTimeBy(400)
    waitForIdle()
    assertEquals(0, words)
    assertEquals(if (outside) 0 else 1, paragraphs)
    if (outside) onNodeWithText("문단 선택").assertIsDisplayed()
    else onNodeWithText("복사").assertIsDisplayed()
  }

  @Test
  fun submenuRevealsVerticallyFromItsTriggerHeight() = runComposeUiTest {
    mainClock.autoAdvance = false
    var bounds = emptyList<Rect>()
    setMenuContent(mode = EditorContextMenuMode.Expanded, onBoundsInWindowChanged = { bounds = it })
    mainClock.advanceTimeBy(400)
    val triggerBounds = onNodeWithText("선택 확장").fetchSemanticsNode().boundsInRoot
    onNodeWithText("선택 확장").performClick()
    val heights = mutableListOf<Float>()
    repeat(24) {
      mainClock.advanceTimeByFrame()
      waitForIdle()
      bounds.getOrNull(1)?.let { heights.add(it.height) }
    }
    assertEquals(triggerBounds.height, heights.first(), 1f, "Must start at the trigger height")
    assertTrue(
      heights.all { it >= triggerBounds.height - 1f },
      "Must never collapse below the trigger: $heights",
    )
    assertTrue(heights.distinct().size >= 4, "Must reveal continuously: $heights")
    assertTrue(
      heights.zipWithNext().all { (a, b) -> b >= a - 1f },
      "Must not shrink during entry: $heights",
    )
  }

  @Test
  fun submenuMovesUpFromTheTriggerAndKeepsItsWidthWhileClosing() = runComposeUiTest {
    mainClock.autoAdvance = false
    var bounds = emptyList<Rect>()
    setMenuContent(
      mode = EditorContextMenuMode.Expanded,
      anchor = EditorContextMenuAnchor(200f, 650f, 650f, atPointer = true),
      onBoundsInWindowChanged = { bounds = it },
    )
    mainClock.advanceTimeBy(400)
    val triggerBounds = onNodeWithText("선택 확장").fetchSemanticsNode().boundsInRoot
    val trigger = triggerBounds.center
    onNodeWithText("선택 확장").performClick()
    val opening = mutableListOf<Rect>()
    repeat(20) {
      mainClock.advanceTimeByFrame()
      waitForIdle()
      bounds.getOrNull(1)?.let(opening::add)
    }
    assertTrue(
      opening.first().top > opening.last().top + 60f,
      "Must move upward while expanding: $opening",
    )
    assertTrue(opening.first().top >= trigger.y - 50f, "Must begin at the trigger: $opening")
    assertTrue(opening.zipWithNext().all { (a, b) -> b.top <= a.top + 1f })
    onNodeWithContentDescription("선택 확장 접기").performClick()
    val closing = mutableListOf<Rect>()
    repeat(20) {
      mainClock.advanceTimeByFrame()
      waitForIdle()
      bounds.getOrNull(1)?.let(closing::add)
    }
    val width = opening.last().width
    assertTrue(closing.size >= 4)
    assertTrue(
      closing.all { kotlin.math.abs(it.width - width) <= 1f },
      "Closing must preserve width: $closing",
    )
    assertEquals(
      triggerBounds.height,
      closing.last().height,
      1f,
      "Must finish at the trigger height",
    )
    assertTrue(
      closing.all { it.height >= triggerBounds.height - 1f },
      "Closing must retain the trigger row",
    )
    assertTrue(closing.last().top > closing.first().top + 60f)
    onNodeWithText("복사").assertIsDisplayed()
  }

  @Test
  fun dismissingFadesTheWholeSubmenuStackAndStopsInputImmediately() = runComposeUiTest {
    mainClock.autoAdvance = false
    var visible by mutableStateOf(true)
    var hidden = 0
    var selections = 0
    var bounds = emptyList<Rect>()
    setMenuContent(
      mode = EditorContextMenuMode.Expanded,
      colors = LightColors.copy(surfaceDefault = Color.Red),
      visible = { visible },
      onHidden = { hidden++ },
      onExpandWord = { selections++ },
      onBoundsInWindowChanged = { bounds = it },
    )
    mainClock.advanceTimeBy(400)
    onNodeWithText("선택 확장").performClick()
    mainClock.advanceTimeBy(400)
    val points = bounds.map { Offset(it.center.x, it.top + 8f) }
    val wordCenter = onNodeWithText("단어 선택").fetchSemanticsNode().boundsInRoot.center
    fun strengths(): List<Float> {
      val pixels = onRoot().captureToImage().toPixelMap()
      return points.map { point ->
        val pixel = pixels[point.x.roundToInt(), point.y.roundToInt()]
        (pixel.red - pixel.green) * pixel.alpha
      }
    }
    val before = strengths()
    assertEquals(2, before.size)
    assertTrue(before.all { it > 0.9f })
    runOnIdle { visible = false }
    mainClock.advanceTimeBy(64)
    assertTrue(bounds.isEmpty(), "The exiting menu must stop claiming outside-tap bounds")
    onNodeWithText("단어 선택").assertDoesNotExist()
    onRoot().performMouseInput { click(wordCenter) }
    assertEquals(0, selections)
    val during = strengths()
    assertTrue(during.all { it in 0.1f..0.9f }, "Both cards must fade together: $during")
    assertEquals(during[0], during[1], absoluteTolerance = 0.03f)
    assertEquals(0, hidden)
    mainClock.advanceTimeBy(120)
    assertTrue(strengths().all { it < 0.01f })
    assertEquals(1, hidden)
  }

  @Test
  fun reopeningDuringExitCancelsRemovalAndRestoresInput() = runComposeUiTest {
    mainClock.autoAdvance = false
    var visible by mutableStateOf(true)
    var hidden = 0
    var copies = 0
    setMenuContent(visible = { visible }, onHidden = { hidden++ }, onCopy = { copies++ })
    mainClock.advanceTimeBy(400)
    runOnIdle { visible = false }
    mainClock.advanceTimeBy(48)
    runOnIdle { visible = true }
    mainClock.advanceTimeBy(240)
    onNodeWithText("복사").performMouseInput { click() }
    assertEquals(1, copies)
    assertEquals(0, hidden)
  }

  @Test
  fun submenuKeepsTheOuterBorderVisibleWhileOpeningAndClosing() = runComposeUiTest {
    mainClock.autoAdvance = false
    var bounds = emptyList<Rect>()
    setMenuContent(
      mode = EditorContextMenuMode.Expanded,
      colors = LightColors.copy(borderDefault = Color.Red),
      onBoundsInWindowChanged = { bounds = it },
    )
    mainClock.advanceTimeBy(400)
    waitForIdle()

    fun outerBorderStrength(rect: Rect): Float {
      val pixels = onRoot().captureToImage().toPixelMap()
      val left = rect.left.roundToInt()
      val y = rect.center.y.roundToInt()
      return ((left - 2)..(left + 3))
        .sumOf { x ->
          val color = pixels[x, y]
          (color.red - color.green).toDouble()
        }
        .toFloat()
    }

    val normalBorderStrength = outerBorderStrength(bounds.single())
    assertTrue(normalBorderStrength > 0.5f)
    val strengths = mutableListOf<Float>()
    onNodeWithText("선택 확장").performClick()
    repeat(8) {
      mainClock.advanceTimeByFrame()
      waitForIdle()
      bounds.getOrNull(1)?.let { strengths.add(outerBorderStrength(it)) }
    }
    mainClock.advanceTimeBy(400)
    onNodeWithContentDescription("선택 확장 접기").performClick()
    repeat(20) {
      mainClock.advanceTimeByFrame()
      waitForIdle()
      bounds.getOrNull(1)?.let { strengths.add(outerBorderStrength(it)) }
    }
    assertTrue(strengths.size >= 8)
    assertTrue(
      strengths.all { it >= normalBorderStrength * 0.8f },
      "An opaque submenu must preserve the outer border: normal=$normalBorderStrength, frames=$strengths",
    )
  }

  @Test
  fun contextualActionsAndCommentsUseTheExpandedMenu() = runComposeUiTest {
    var downloads = 0
    var comments = 0
    var dismisses = 0
    setMenuContent(
      editorMutationEnabled = false,
      contextualItems = listOf(EditorContextMenuItem("이미지 내려받기", Lucide.Download, { downloads++ })),
      onComment = { comments++ },
      onDismiss = { dismisses++ },
    )
    assertEquals(0, onAllNodesWithText("코멘트 달기").fetchSemanticsNodes().size)
    onNodeWithContentDescription("메뉴 펼치기").performClick()
    onNodeWithText("이미지 내려받기").performMouseInput { click() }
    onNodeWithText("코멘트 달기").performTouchInput { click() }
    assertEquals(1, downloads)
    assertEquals(1, comments)
    assertEquals(2, dismisses)
  }

  @Test
  fun compactMenuExpandsToAVerticalMenuWithASelectionSubmenu() = runComposeUiTest {
    setMenuContent()

    onNodeWithContentDescription("메뉴 펼치기").performClick()
    waitForIdle()

    onNodeWithText("복사").assertIsDisplayed()
    onNodeWithText("선택 확장").assertIsDisplayed()
    assertEquals(0, onAllNodesWithText("단어 선택").fetchSemanticsNodes().size)
    val copy = onNodeWithText("복사").fetchSemanticsNode().boundsInRoot
    val cut = onNodeWithText("잘라내기").fetchSemanticsNode().boundsInRoot
    assertTrue(cut.top >= copy.bottom)
    onNodeWithText("선택 확장").performClick()
    onNodeWithText("단어 선택").assertIsDisplayed()
    onNodeWithText("전체 선택").assertIsDisplayed()
  }

  @Test
  fun expansionMovesSmoothlyWhenTheFullMenuMustSwitchBelowTheSelection() = runComposeUiTest {
    mainClock.autoAdvance = false
    var bounds: Rect? = null
    setMenuContent(
      anchor = EditorContextMenuAnchor(200f, 100f, 320f),
      onBoundsInWindowChanged = { bounds = it.firstOrNull() },
    )
    mainClock.advanceTimeBy(300)
    waitForIdle()
    val initialTop = requireNotNull(bounds).top
    onNodeWithContentDescription("메뉴 펼치기").performClick()

    val positions = mutableListOf(initialTop)
    repeat(15) {
      mainClock.advanceTimeByFrame()
      waitForIdle()
      positions.add(requireNotNull(bounds).top)
    }
    assertTrue(positions.last() > initialTop)
    assertTrue(
      positions.zipWithNext().all { (previous, next) -> next >= previous - 1f },
      "Menu should move toward its final placement without flipping midway: $positions",
    )
  }

  @Test
  fun compactMenuScrollsLargeLabelsWhileKeepingNavigationButtonsVisible() = runComposeUiTest {
    var selections = 0
    setMenuContent(overlaySize = Size(240f, 700f), fontScale = 2f, onSelectAll = { selections++ })
    onNodeWithText("선택 확장").performScrollTo()
    val expand = onNodeWithContentDescription("메뉴 펼치기").fetchSemanticsNode().boundsInRoot
    assertTrue(expand.width >= 48f && expand.height > 48f && expand.height < 100f)
    assertTrue(expand.right <= 240f)
    onNodeWithText("복사").performScrollTo()
    onNode(hasScrollAction()).performTouchInput { swipeLeft() }
    assertTrue(
      onNode(hasScrollAction())
        .fetchSemanticsNode()
        .config[SemanticsProperties.HorizontalScrollAxisRange]
        .value() > 0f
    )
    assertEquals(0, selections)
    assertEquals(expand, onNodeWithContentDescription("메뉴 펼치기").fetchSemanticsNode().boundsInRoot)
    onNodeWithText("선택 확장").performScrollTo().performClick()
    val back = onNodeWithContentDescription("이전").fetchSemanticsNode().boundsInRoot
    onNodeWithContentDescription("메뉴 펼치기").assertIsDisplayed()
    onNodeWithText("전체").performScrollTo()
    assertEquals(back, onNodeWithContentDescription("이전").fetchSemanticsNode().boundsInRoot)
    onNodeWithContentDescription("메뉴 펼치기").assertIsDisplayed()
    onNodeWithText("전체").performTouchInput { click() }
    assertEquals(1, selections)
  }

  @Test
  fun compactMenuKeepsShortContentNarrowAndBothButtonsEasyToTap() = runComposeUiTest {
    var bounds = emptyList<Rect>()
    setMenuContent(
      editorMutationEnabled = false,
      availableExpansionUnits = emptySet(),
      onBoundsInWindowChanged = { bounds = it },
    )
    val copy = onNodeWithText("복사").fetchSemanticsNode().boundsInRoot
    val expand = onNodeWithContentDescription("메뉴 펼치기").fetchSemanticsNode().boundsInRoot
    assertTrue(copy.width >= 48f && copy.height >= 48f)
    assertTrue(expand.width >= 48f && expand.height >= 48f)
    assertTrue(bounds.single().width < 200f, "Short menus must not stretch across the viewport")
  }

  @Test
  fun compactMenuKeepsItsExistingSelectionPage() = runComposeUiTest {
    var words = 0
    setMenuContent(onExpandWord = { words++ })
    onNodeWithText("선택 확장").performClick()
    onNodeWithText("단어").assertIsDisplayed()
    onNodeWithText("문장").assertIsDisplayed()
    onNodeWithText("문단").assertIsDisplayed()
    onNodeWithText("전체").assertIsDisplayed()
    onNodeWithContentDescription("이전").performClick()
    onNodeWithText("복사").assertIsDisplayed()
    onNodeWithText("선택 확장").performClick()
    onNodeWithText("단어").performClick()
    assertEquals(1, words)
    onNodeWithText("복사").assertIsDisplayed()
  }

  @Test
  fun expandedMenuShowsOnlyAvailableSelectionActions() = runComposeUiTest {
    setMenuContent(
      mode = EditorContextMenuMode.Expanded,
      availableExpansionUnits = setOf(SelectionExpansionUnit.Word, SelectionExpansionUnit.Paragraph),
    )
    onNodeWithText("선택 확장").performClick()
    onNodeWithText("단어 선택").assertIsDisplayed()
    onNodeWithText("문단 선택").assertIsDisplayed()
    assertEquals(0, onAllNodesWithText("문장 선택").fetchSemanticsNodes().size)
    assertEquals(0, onAllNodesWithText("전체 선택").fetchSemanticsNodes().size)
  }

  @Test
  fun expandedSubmenuCanCollapseWithoutActivatingTheParentItem() = runComposeUiTest {
    var copies = 0
    setMenuContent(mode = EditorContextMenuMode.Expanded, onCopy = { copies++ })
    val copyCenter = onNodeWithText("복사").fetchSemanticsNode().boundsInRoot.center
    onNodeWithText("선택 확장").performClick()
    onNodeWithText("단어 선택").assertIsDisplayed()
    onRoot().performMouseInput { click(copyCenter) }
    waitForIdle()
    assertEquals(0, copies)
    onNodeWithText("복사").assertIsDisplayed()
    assertEquals(0, onAllNodesWithText("단어 선택").fetchSemanticsNodes().size)
    onNodeWithText("선택 확장").performClick()
    onNodeWithContentDescription("선택 확장 접기").performClick()
    onNodeWithText("복사").performMouseInput { click() }
    assertEquals(1, copies)
  }

  @Test
  fun compactMenuCanExpandWithoutSelectionActions() = runComposeUiTest {
    setMenuContent(availableExpansionUnits = emptySet())

    waitForIdle()

    onNodeWithContentDescription("메뉴 펼치기").performClick()
    waitForIdle()
    onNodeWithText("복사").assertIsDisplayed()
    assertEquals(0, onAllNodesWithText("전체 선택").fetchSemanticsNodes().size)
  }

  @Test
  fun collapsedMenuHidesRangeOnlyActionsButKeepsSelectionExpansion() = runComposeUiTest {
    setMenuContent(showCopyCutActions = false)

    waitForIdle()

    assertEquals(0, onAllNodesWithText("복사").fetchSemanticsNodes().size)
    assertEquals(0, onAllNodesWithText("잘라내기").fetchSemanticsNodes().size)
    assertEquals(1, onAllNodesWithText("붙여넣기").fetchSemanticsNodes().size)
    onNodeWithContentDescription("메뉴 펼치기").assertIsDisplayed()
  }

  @Test
  fun disabledMutationMenuHidesCutAndPasteButKeepsCopyAndExpansion() = runComposeUiTest {
    setMenuContent(editorMutationEnabled = false)

    waitForIdle()

    assertEquals(1, onAllNodesWithText("복사").fetchSemanticsNodes().size)
    assertEquals(0, onAllNodesWithText("잘라내기").fetchSemanticsNodes().size)
    assertEquals(0, onAllNodesWithText("붙여넣기").fetchSemanticsNodes().size)
    onNodeWithContentDescription("메뉴 펼치기").assertIsDisplayed()
  }

  @Test
  fun enabledMutationMenuKeepsCompletePrimaryMenu() = runComposeUiTest {
    setMenuContent(editorMutationEnabled = true)

    waitForIdle()

    assertEquals(1, onAllNodesWithText("복사").fetchSemanticsNodes().size)
    assertEquals(1, onAllNodesWithText("잘라내기").fetchSemanticsNodes().size)
    assertEquals(1, onAllNodesWithText("붙여넣기").fetchSemanticsNodes().size)
    onNodeWithContentDescription("메뉴 펼치기").assertIsDisplayed()
  }

  @Test
  fun expandedMenuScrollsWithinTheUnoccludedViewport() = runComposeUiTest {
    setMenuContent(
      mode = EditorContextMenuMode.Expanded,
      visibleArea =
        EditorVisibleArea(viewport = Size(400f, 700f), topInset = 100f, safeBottomInset = 380f),
    )

    onNodeWithText("선택 확장").performClick()
    onNodeWithText("전체 선택").performScrollTo().assertIsDisplayed()
    val lastItem = onNodeWithText("전체 선택").fetchSemanticsNode().boundsInRoot
    assertTrue(lastItem.top >= 104f)
    assertTrue(lastItem.bottom <= 316f)
  }

  @Test
  fun expandedReadOnlyMenuKeepsCopyAndSelectionActions() = runComposeUiTest {
    setMenuContent(mode = EditorContextMenuMode.Expanded, editorMutationEnabled = false)

    onNodeWithText("복사").assertIsDisplayed()
    onNodeWithText("선택 확장").performClick()
    onNodeWithText("전체 선택").assertIsDisplayed()
    assertEquals(0, onAllNodesWithText("잘라내기").fetchSemanticsNodes().size)
    assertEquals(0, onAllNodesWithText("붙여넣기").fetchSemanticsNodes().size)
  }

  @Test
  fun primaryMenuItemInvokesActionAndDismisses() = runComposeUiTest {
    var copyCount = 0
    var dismissCount = 0
    setMenuContent(onCopy = { copyCount++ }, onDismiss = { dismissCount++ })

    waitForIdle()
    onNodeWithText("복사").performClick()
    waitForIdle()

    assertEquals(1, copyCount)
    assertEquals(1, dismissCount)
  }

  @Test
  fun expandedMenuItemInvokesActionAndDismissesOnceForMouseAndTouch() = runComposeUiTest {
    var copyCount = 0
    var dismissCount = 0
    setMenuContent(
      mode = EditorContextMenuMode.Expanded,
      onCopy = { copyCount++ },
      onDismiss = { dismissCount++ },
    )

    onNodeWithText("복사").performMouseInput { click() }
    waitForIdle()
    assertEquals(1, copyCount)
    assertEquals(1, dismissCount)

    onNodeWithText("복사").performTouchInput { click() }
    waitForIdle()
    assertEquals(2, copyCount)
    assertEquals(2, dismissCount)
  }

  @Test
  fun repasteOverlayInvokesRepasteAsText() = runComposeUiTest {
    var repasteCount = 0
    setRepasteContent(onRepasteAsText = { repasteCount += 1 })

    waitForIdle()
    onNodeWithText("서식 없이 다시 붙여넣기").performClick()
    waitForIdle()

    assertEquals(1, repasteCount)
  }

  @Test
  fun repasteOverlayShowsWithoutCursorWhenVisible() = runComposeUiTest {
    setRepasteContent()

    waitForIdle()
    assertEquals(1, onAllNodesWithText("서식 없이 다시 붙여넣기").fetchSemanticsNodes().size)
  }

  @Test
  fun repasteOverlayStaysNearTopWhenBottomIsOccluded() = runComposeUiTest {
    setRepasteContent(
      visibleArea =
        EditorVisibleArea(
          viewport = Size(width = 400f, height = 700f),
          topInset = 24f,
          bottomOcclusionInset = 280f,
        )
    )

    waitForIdle()
    val top = onNodeWithText("서식 없이 다시 붙여넣기").fetchSemanticsNode().boundsInRoot.top
    assertTrue(top < 120f, "expected top placement, got y=$top")
  }

  private fun androidx.compose.ui.test.ComposeUiTest.setMenuContent(
    anchor: EditorContextMenuAnchor = EditorContextMenuAnchor(200f, 220f, 320f),
    overlaySize: Size = Size(400f, 700f),
    fontScale: Float = 1f,
    showCopyCutActions: Boolean = true,
    colors: AppColors = LightColors,
    mode: EditorContextMenuMode = EditorContextMenuMode.Compact,
    visibleArea: EditorVisibleArea = EditorVisibleArea(viewport = Size(400f, 700f)),
    editorMutationEnabled: Boolean = true,
    onCopy: () -> Unit = {},
    onCut: () -> Unit = {},
    onPaste: () -> Unit = {},
    onExpandWord: () -> Unit = {},
    onExpandSentence: () -> Unit = {},
    onExpandParagraph: () -> Unit = {},
    onSelectAll: () -> Unit = {},
    onDismiss: () -> Unit = {},
    visible: () -> Boolean = { true },
    onHidden: () -> Unit = {},
    contextualItems: List<EditorContextMenuItem> = emptyList(),
    onComment: (() -> Unit)? = null,
    onBoundsInWindowChanged: (List<Rect>) -> Unit = {},
    availableExpansionUnits: Set<SelectionExpansionUnit> = SelectionExpansionUnit.entries.toSet(),
  ) {
    setContent {
      var currentMode by remember { mutableStateOf(mode) }
      CompositionLocalProvider(
        LocalDensity provides Density(density = 1f, fontScale = fontScale),
        LocalAppColors provides colors,
        LocalAppShadows provides LightAppShadows,
        LocalThemeMode provides ResolvedThemeMode.Light,
      ) {
        EditorSelectionContextMenuOverlay(
          visible = visible(),
          onHidden = onHidden,
          mode = currentMode,
          onExpandMenu = { currentMode = EditorContextMenuMode.Expanded },
          anchor = anchor,
          overlaySize = overlaySize,
          visibleArea = visibleArea,
          editorMutationEnabled = editorMutationEnabled,
          actions =
            EditorContextMenuActions(
              showCopyCutActions = showCopyCutActions,
              availableExpansionUnits = availableExpansionUnits,
              onCopy = onCopy,
              onCut = onCut,
              onPaste = onPaste,
              onExpandWord = onExpandWord,
              onExpandSentence = onExpandSentence,
              onExpandParagraph = onExpandParagraph,
              onSelectAll = onSelectAll,
              onDismiss = onDismiss,
              contextualItems = contextualItems,
              onComment = onComment,
            ),
          onBoundsInWindowChanged = onBoundsInWindowChanged,
        )
      }
    }
  }

  private fun androidx.compose.ui.test.ComposeUiTest.setRepasteContent(
    visibleArea: EditorVisibleArea =
      EditorVisibleArea(viewport = Size(width = 400f, height = 700f)),
    onRepasteAsText: () -> Unit = {},
  ) {
    setContent {
      CompositionLocalProvider(
        LocalAppColors provides LightColors,
        LocalAppShadows provides LightAppShadows,
        LocalThemeMode provides ResolvedThemeMode.Light,
      ) {
        EditorRepasteAsTextOverlay(
          visibleArea = visibleArea,
          visible = true,
          onRepasteAsText = onRepasteAsText,
        )
      }
    }
  }
}
