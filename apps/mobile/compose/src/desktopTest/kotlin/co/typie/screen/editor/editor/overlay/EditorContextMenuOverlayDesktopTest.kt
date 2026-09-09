package co.typie.screen.editor.editor.overlay

import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.size
import androidx.compose.runtime.CompositionLocalProvider
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.geometry.Size
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.test.ExperimentalTestApi
import androidx.compose.ui.test.click
import androidx.compose.ui.test.onAllNodesWithText
import androidx.compose.ui.test.onNodeWithTag
import androidx.compose.ui.test.onNodeWithText
import androidx.compose.ui.test.performClick
import androidx.compose.ui.test.performMouseInput
import androidx.compose.ui.test.performTouchInput
import androidx.compose.ui.test.runComposeUiTest
import androidx.compose.ui.unit.dp
import co.typie.editor.EditorState
import co.typie.editor.PagePoint
import co.typie.editor.ffi.SelectionExpansionUnit
import co.typie.editor.interaction.EditorInteractionGeometry
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
import co.typie.ui.theme.LightAppShadows
import co.typie.ui.theme.LightColors
import co.typie.ui.theme.LocalAppColors
import co.typie.ui.theme.LocalAppShadows
import co.typie.ui.theme.LocalThemeMode
import co.typie.ui.theme.ResolvedThemeMode
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertFalse
import kotlin.test.assertTrue

@OptIn(ExperimentalTestApi::class)
class EditorContextMenuOverlayDesktopTest {
  @Test
  fun onlyPointerMenusConsumeTheFirstOutsideControlClick() = runComposeUiTest {
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
              anchor = EditorContextMenuAnchor(200f, 220f, 320f),
              overlaySize = Size(400f, 700f),
              visibleArea = EditorVisibleArea(viewport = Size(400f, 700f)),
              showCopyCutActions = true,
              availableExpansionUnits = SelectionExpansionUnit.entries.toSet(),
              onCopy = {},
              onCut = {},
              onPaste = {},
              onExpandWord = {},
              onExpandSentence = {},
              onExpandParagraph = {},
              onSelectAll = {},
              onDismiss = menu::hide,
              onBoundsInWindowChanged = { menu.boundsInWindow = it },
            )
          }
        }
      }
    }
    for (pointerMenu in listOf(true, false)) {
      repeat(2) { index ->
        val before = clicks
        runOnIdle {
          menu.show(EditorState.Initial, if (pointerMenu) PagePoint(0, 200f, 220f) else null)
        }
        if (index == 0) onNodeWithTag("outside").performMouseInput { click() }
        else onNodeWithTag("outside").performTouchInput { click() }
        waitForIdle()
        assertFalse(menu.visible)
        assertEquals(before + if (pointerMenu) 0 else 1, clicks)
      }
    }
    val before = clicks
    onNodeWithTag("outside").performMouseInput { click() }
    waitForIdle()
    assertEquals(before + 1, clicks)
    for (pointerMenu in listOf(true, false)) {
      runOnIdle {
        menu.show(EditorState.Initial, if (pointerMenu) PagePoint(0, 200f, 220f) else null)
      }
      onNodeWithTag("indicator").performMouseInput { click(Offset(width - 5f, center.y)) }
      waitForIdle()
      assertFalse(menu.visible)
      assertEquals(if (pointerMenu) 0 else 1, selectedPage)
    }
    for (pointerMenu in listOf(true, false)) {
      runOnIdle {
        menu.show(EditorState.Initial, if (pointerMenu) PagePoint(0, 200f, 220f) else null)
      }
      onNodeWithTag("anchor").performMouseInput { click() }
      waitForIdle()
      assertFalse(menu.visible)
      assertEquals(!pointerMenu, popover.acceptsInput)
    }
  }

  @Test
  fun selectingExpansionShowsLegacyExpansionMenu() = runComposeUiTest {
    setMenuContent()

    waitForIdle()
    assertEquals(0, onAllNodesWithText("단어").fetchSemanticsNodes().size)

    onNodeWithText("선택 확장").performClick()
    waitForIdle()

    assertEquals(0, onAllNodesWithText("선택 확장").fetchSemanticsNodes().size)
    assertEquals(1, onAllNodesWithText("단어").fetchSemanticsNodes().size)
    assertEquals(1, onAllNodesWithText("문장").fetchSemanticsNodes().size)
    assertEquals(1, onAllNodesWithText("문단").fetchSemanticsNodes().size)
    assertEquals(1, onAllNodesWithText("전체").fetchSemanticsNodes().size)
  }

  @Test
  fun selectingExpansionShowsOnlyAvailableExpansionActions() = runComposeUiTest {
    setMenuContent(
      availableExpansionUnits = setOf(SelectionExpansionUnit.Word, SelectionExpansionUnit.Paragraph)
    )

    waitForIdle()
    onNodeWithText("선택 확장").performClick()
    waitForIdle()

    assertEquals(1, onAllNodesWithText("단어").fetchSemanticsNodes().size)
    assertEquals(0, onAllNodesWithText("문장").fetchSemanticsNodes().size)
    assertEquals(1, onAllNodesWithText("문단").fetchSemanticsNodes().size)
    assertEquals(0, onAllNodesWithText("전체").fetchSemanticsNodes().size)
  }

  @Test
  fun primaryMenuHidesSelectionExpansionWhenNoUnitIsAvailable() = runComposeUiTest {
    setMenuContent(availableExpansionUnits = emptySet())

    waitForIdle()

    assertEquals(0, onAllNodesWithText("선택 확장").fetchSemanticsNodes().size)
  }

  @Test
  fun collapsedMenuHidesRangeOnlyActionsButKeepsSelectionExpansion() = runComposeUiTest {
    setMenuContent(showCopyCutActions = false)

    waitForIdle()

    assertEquals(0, onAllNodesWithText("복사").fetchSemanticsNodes().size)
    assertEquals(0, onAllNodesWithText("잘라내기").fetchSemanticsNodes().size)
    assertEquals(1, onAllNodesWithText("붙여넣기").fetchSemanticsNodes().size)
    assertEquals(1, onAllNodesWithText("선택 확장").fetchSemanticsNodes().size)
  }

  @Test
  fun disabledMutationMenuHidesCutAndPasteButKeepsCopyAndExpansion() = runComposeUiTest {
    setMenuContent(editorMutationEnabled = false)

    waitForIdle()

    assertEquals(1, onAllNodesWithText("복사").fetchSemanticsNodes().size)
    assertEquals(0, onAllNodesWithText("잘라내기").fetchSemanticsNodes().size)
    assertEquals(0, onAllNodesWithText("붙여넣기").fetchSemanticsNodes().size)
    assertEquals(1, onAllNodesWithText("선택 확장").fetchSemanticsNodes().size)
  }

  @Test
  fun enabledMutationMenuKeepsCompletePrimaryMenu() = runComposeUiTest {
    setMenuContent(editorMutationEnabled = true)

    waitForIdle()

    assertEquals(1, onAllNodesWithText("복사").fetchSemanticsNodes().size)
    assertEquals(1, onAllNodesWithText("잘라내기").fetchSemanticsNodes().size)
    assertEquals(1, onAllNodesWithText("붙여넣기").fetchSemanticsNodes().size)
    assertEquals(1, onAllNodesWithText("선택 확장").fetchSemanticsNodes().size)
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
    showCopyCutActions: Boolean = true,
    editorMutationEnabled: Boolean = true,
    onCopy: () -> Unit = {},
    onCut: () -> Unit = {},
    onPaste: () -> Unit = {},
    onExpandWord: () -> Unit = {},
    onExpandSentence: () -> Unit = {},
    onExpandParagraph: () -> Unit = {},
    onSelectAll: () -> Unit = {},
    onDismiss: () -> Unit = {},
    availableExpansionUnits: Set<SelectionExpansionUnit> = SelectionExpansionUnit.entries.toSet(),
  ) {
    setContent {
      CompositionLocalProvider(
        LocalAppColors provides LightColors,
        LocalAppShadows provides LightAppShadows,
        LocalThemeMode provides ResolvedThemeMode.Light,
      ) {
        EditorSelectionContextMenuOverlay(
          anchor = EditorContextMenuAnchor(centerX = 200f, above = 220f, below = 320f),
          overlaySize = Size(width = 400f, height = 700f),
          visibleArea = EditorVisibleArea(viewport = Size(width = 400f, height = 700f)),
          showCopyCutActions = showCopyCutActions,
          editorMutationEnabled = editorMutationEnabled,
          availableExpansionUnits = availableExpansionUnits,
          onCopy = onCopy,
          onCut = onCut,
          onPaste = onPaste,
          onExpandWord = onExpandWord,
          onExpandSentence = onExpandSentence,
          onExpandParagraph = onExpandParagraph,
          onSelectAll = onSelectAll,
          onDismiss = onDismiss,
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
