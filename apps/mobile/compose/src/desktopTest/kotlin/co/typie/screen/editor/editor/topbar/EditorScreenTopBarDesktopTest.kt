package co.typie.screen.editor.editor.topbar

import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.size
import androidx.compose.runtime.Composable
import androidx.compose.runtime.CompositionLocalProvider
import androidx.compose.runtime.mutableStateListOf
import androidx.compose.runtime.remember
import androidx.compose.ui.Modifier
import androidx.compose.ui.geometry.Rect
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.test.ExperimentalTestApi
import androidx.compose.ui.test.click
import androidx.compose.ui.test.hasAnyDescendant
import androidx.compose.ui.test.hasClickAction
import androidx.compose.ui.test.hasText
import androidx.compose.ui.test.onNodeWithContentDescription
import androidx.compose.ui.test.onNodeWithTag
import androidx.compose.ui.test.onNodeWithText
import androidx.compose.ui.test.performClick
import androidx.compose.ui.test.performTouchInput
import androidx.compose.ui.test.runComposeUiTest
import androidx.compose.ui.unit.dp
import co.typie.navigation.LocalRoute
import co.typie.navigation.Nav
import co.typie.navigation.Navigator
import co.typie.route.Route
import co.typie.screen.editor.editor.toolbar.EditorToolbarDebugOverlays
import co.typie.screen.editor.editor.toolbar.EditorToolbarToolAction
import co.typie.ui.component.popover.LocalPopoverOverlayState
import co.typie.ui.component.popover.PopoverOverlay
import co.typie.ui.component.popover.PopoverOverlayState
import co.typie.ui.component.topbar.LocalTopBarState
import co.typie.ui.component.topbar.TopBar
import co.typie.ui.component.topbar.TopBarDefaults
import co.typie.ui.component.topbar.TopBarState
import co.typie.ui.theme.LightAppShadows
import co.typie.ui.theme.LightColors
import co.typie.ui.theme.LocalAppColors
import co.typie.ui.theme.LocalAppShadows
import co.typie.ui.theme.LocalThemeMode
import co.typie.ui.theme.ResolvedThemeMode
import dev.chrisbanes.haze.blur.HazeBlurStyle
import dev.chrisbanes.haze.blur.LocalHazeBlurStyle
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertFalse
import kotlin.test.assertTrue

@OptIn(ExperimentalTestApi::class)
class EditorScreenTopBarDesktopTest {
  @Test
  fun readingShowsSearchAndToolsAndDispatchesEveryToolAction() = runComposeUiTest {
    val actions = mutableStateListOf<EditorToolbarToolAction>()
    val overlayState = PopoverOverlayState()

    setTopBarContent(editing = false, overlayState = overlayState, onToolAction = actions::add)

    onNodeWithContentDescription(SearchDescription).assertExists()
    onNodeWithContentDescription(ToolsDescription).assertExists()
    onNodeWithContentDescription(EnterReadingDescription).assertDoesNotExist()

    onNodeWithContentDescription(SearchDescription).performClick()
    waitUntil { actions.lastOrNull() == EditorToolbarToolAction.Search }

    val expectedItems =
      listOf(
        "노트" to EditorToolbarToolAction.RelatedNotes,
        "코멘트" to EditorToolbarToolAction.Comment,
        "맞춤법 검사" to EditorToolbarToolAction.Spellcheck,
        "AI 피드백" to EditorToolbarToolAction.AiFeedback,
        "타임라인" to EditorToolbarToolAction.Timeline,
      )
    expectedItems.forEach { (label, action) ->
      openTools(overlayState)
      onAllNodes(hasClickAction() and hasAnyDescendant(hasText(label)), useUnmergedTree = true)[0]
        .performClick()
      waitUntil { actions.lastOrNull() == action }
    }

    assertEquals(
      listOf(EditorToolbarToolAction.Search) + expectedItems.map { it.second },
      actions.toList(),
    )
    onNodeWithText("뷰포트 기준선 켜기", useUnmergedTree = true).assertDoesNotExist()
  }

  @Test
  fun editingShowsOnlyReadingModeButton() = runComposeUiTest {
    var enterReadingCount = 0

    setTopBarContent(editing = true, onEnterReadingMode = { enterReadingCount += 1 })

    onNodeWithContentDescription(EnterReadingDescription).assertExists().performClick()
    waitUntil { enterReadingCount == 1 }
    onNodeWithContentDescription(SearchDescription).assertDoesNotExist()
    onNodeWithContentDescription(ToolsDescription).assertDoesNotExist()
  }

  @Test
  fun editorTopBarDisablesBackdropBlur() = runComposeUiTest {
    val topBarState = setTopBarContent(editing = false)

    assertFalse(topBarState.backdropBlurEnabled)
  }

  @Test
  fun debugToolsAppearOnlyWhenDebugMetadataIsSupplied() = runComposeUiTest {
    val actions = mutableStateListOf<EditorToolbarToolAction>()
    val overlayState = PopoverOverlayState()

    setTopBarContent(
      editing = false,
      overlayState = overlayState,
      debugOverlays =
        EditorToolbarDebugOverlays(
          viewportVisible = false,
          bodyVisible = true,
          surfaceVisible = false,
          inputLogAvailable = true,
        ),
      onToolAction = actions::add,
    )

    val expectedItems =
      listOf(
        "뷰포트 기준선 켜기" to EditorToolbarToolAction.DebugViewportOverlay,
        "바디 영역 끄기" to EditorToolbarToolAction.DebugBodyOverlay,
        "페이지 표면 켜기" to EditorToolbarToolAction.DebugSurfaceOverlay,
        "입력 로그 보내기" to EditorToolbarToolAction.SendInputLog,
      )
    expectedItems.forEach { (label, action) ->
      openTools(overlayState)
      onAllNodes(hasClickAction() and hasAnyDescendant(hasText(label)), useUnmergedTree = true)[0]
        .performClick()
      waitUntil { actions.lastOrNull() == action }
    }

    assertEquals(expectedItems.map { it.second }, actions.toList())
  }

  @Test
  fun readingTitleUsesAvailableSpaceWithoutIntersectingTrailingButtons() = runComposeUiTest {
    setTopBarContent(editing = false, width = 360.dp)

    val rootBounds = onNodeWithTag(RootTag).fetchSemanticsNode().boundsInRoot
    val titleBounds = onNodeWithTag(TitleTag).fetchSemanticsNode().boundsInRoot
    val searchBounds =
      onNodeWithContentDescription(SearchDescription).fetchSemanticsNode().boundsInRoot
    val toolsBounds =
      onNodeWithContentDescription(ToolsDescription).fetchSemanticsNode().boundsInRoot

    assertTrue(titleBounds.center.x < rootBounds.center.x)
    assertFalse(titleBounds.intersects(searchBounds))
    assertFalse(titleBounds.intersects(toolsBounds))
  }

  private fun androidx.compose.ui.test.ComposeUiTest.openTools(overlayState: PopoverOverlayState) {
    onNodeWithContentDescription(ToolsDescription).performTouchInput { click() }
    waitUntil { overlayState.acceptsInput }
  }

  private fun androidx.compose.ui.test.ComposeUiTest.setTopBarContent(
    editing: Boolean,
    width: androidx.compose.ui.unit.Dp = 400.dp,
    overlayState: PopoverOverlayState = PopoverOverlayState(),
    debugOverlays: EditorToolbarDebugOverlays? = null,
    onToolAction: (EditorToolbarToolAction) -> Unit = {},
    onEnterReadingMode: suspend () -> Unit = {},
  ): TopBarState {
    lateinit var capturedTopBarState: TopBarState
    setContent {
      TopBarTestTheme {
        val topBarState = remember { TopBarState() }
        capturedTopBarState = topBarState
        CompositionLocalProvider(
          Nav provides Navigator(Route.Home),
          LocalRoute provides Route.Editor("document-id"),
          LocalTopBarState provides topBarState,
          LocalPopoverOverlayState provides overlayState,
        ) {
          Box(Modifier.size(width = width, height = 700.dp).testTag(RootTag)) {
            EditorScreenTopBar(
              editing = editing,
              debugOverlays = debugOverlays,
              documentButton = { modifier ->
                Box(modifier.height(TopBarDefaults.TitleHeight).testTag(TitleTag))
              },
              onToolAction = onToolAction,
              onEnterReadingMode = onEnterReadingMode,
            )
            TopBar(state = topBarState)
            PopoverOverlay(state = overlayState)
          }
        }
      }
    }
    waitForIdle()
    return capturedTopBarState
  }

  @Composable
  private fun TopBarTestTheme(content: @Composable () -> Unit) {
    CompositionLocalProvider(
      LocalAppColors provides LightColors,
      LocalAppShadows provides LightAppShadows,
      LocalThemeMode provides ResolvedThemeMode.Light,
      LocalHazeBlurStyle provides
        HazeBlurStyle {
          blurRadius(20.dp)
          noiseFactor(0f)
          colorEffects(emptyList())
        },
      content = content,
    )
  }

  private companion object {
    const val RootTag = "editor-top-bar-root"
    const val TitleTag = "editor-top-bar-title"
    const val SearchDescription = "검색"
    const val ToolsDescription = "도구"
    const val EnterReadingDescription = "읽기 모드로 전환"
  }
}

private fun Rect.intersects(other: Rect): Boolean =
  left < other.right && right > other.left && top < other.bottom && bottom > other.top
