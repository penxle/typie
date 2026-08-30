package co.typie.ui.component.bottombar

import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.WindowInsets
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.runtime.Composable
import androidx.compose.runtime.CompositionLocalProvider
import androidx.compose.runtime.SideEffect
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.platform.LocalDensity
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.test.ComposeUiTest
import androidx.compose.ui.test.ExperimentalTestApi
import androidx.compose.ui.test.hasClickAction
import androidx.compose.ui.test.onNodeWithTag
import androidx.compose.ui.test.onNodeWithText
import androidx.compose.ui.test.performTouchInput
import androidx.compose.ui.test.v2.runComposeUiTest
import androidx.compose.ui.unit.dp
import co.typie.dev.DesktopDebugKeyboard
import co.typie.ext.ime
import co.typie.ext.navigationBars
import co.typie.ext.navigationBarsPadding
import co.typie.icons.Lucide
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
import kotlin.test.assertTrue

@OptIn(ExperimentalTestApi::class)
class BottomBarActionButtonDesktopTest {
  @Test
  fun actionFollowsLiveImeWhilePillRemainsAnchored() = runComposeUiTest {
    val state =
      BottomBarState().apply {
        enabled = true
        setPill(PillKey) {
          Box(Modifier.fillMaxSize(), contentAlignment = Alignment.BottomCenter) {
            Box(
              Modifier.navigationBarsPadding().padding(bottom = BottomBarDefaults.BottomPadding)
            ) {
              Box(
                Modifier.testTag(PillTag)
                  .size(width = 220.dp, height = BottomBarDefaults.PillHeight)
              )
            }
          }
        }
        setAction(ActionKey, BottomBarActionEntry.Data(BottomBarAction(icon = Lucide.Plus)))
      }
    val keyboardOwner = Any()
    val previousHardwareKeyboardConnected = DesktopDebugKeyboard.hardwareKeyboardConnected
    var navigationBarsBottomPx = 0f
    var imeBottomPx = 0f
    var bottomPaddingPx = 0f
    var imeGapPx = 0f

    try {
      setContent {
        BottomBarTestTheme {
          val density = LocalDensity.current
          val navigationBars = WindowInsets.navigationBars
          val ime = WindowInsets.ime
          SideEffect {
            navigationBarsBottomPx = navigationBars.getBottom(density).toFloat()
            imeBottomPx = ime.getBottom(density).toFloat()
            bottomPaddingPx = with(density) { BottomBarDefaults.BottomPadding.toPx() }
            imeGapPx = with(density) { ImeGap.toPx() }
          }

          Box(Modifier.testTag(RootTag).size(width = 400.dp, height = 700.dp)) { BottomBar(state) }
        }
      }

      runOnIdle {
        DesktopDebugKeyboard.updateHardwareKeyboardConnected(false)
        DesktopDebugKeyboard.hideKeyboardSurface()
      }
      mainClock.advanceTimeBy(300)
      waitForIdle()

      val rootBounds = onNodeWithTag(RootTag).fetchSemanticsNode().boundsInRoot
      val initialActionBounds = onNode(hasClickAction()).fetchSemanticsNode().boundsInRoot
      val initialPillBounds = onNodeWithTag(PillTag).fetchSemanticsNode().boundsInRoot
      assertEquals(0f, imeBottomPx, absoluteTolerance = PositionTolerancePx)
      assertEquals(
        rootBounds.bottom - maxOf(navigationBarsBottomPx + bottomPaddingPx, imeGapPx),
        initialActionBounds.bottom,
        absoluteTolerance = PositionTolerancePx,
      )

      mainClock.autoAdvance = false
      runOnIdle { DesktopDebugKeyboard.notifyFocusChanged(keyboardOwner, isFocused = true) }
      mainClock.advanceTimeBy(110)
      waitForIdle()

      val intermediateActionBounds = onNode(hasClickAction()).fetchSemanticsNode().boundsInRoot
      val intermediatePillBounds = onNodeWithTag(PillTag).fetchSemanticsNode().boundsInRoot
      assertTrue(imeBottomPx > 0f)
      assertEquals(
        rootBounds.bottom - maxOf(navigationBarsBottomPx + bottomPaddingPx, imeBottomPx + imeGapPx),
        intermediateActionBounds.bottom,
        absoluteTolerance = PositionTolerancePx,
      )
      assertEquals(initialPillBounds, intermediatePillBounds)

      mainClock.advanceTimeBy(200)
      waitForIdle()

      val settledActionBounds = onNode(hasClickAction()).fetchSemanticsNode().boundsInRoot
      val settledPillBounds = onNodeWithTag(PillTag).fetchSemanticsNode().boundsInRoot
      assertEquals(
        imeGapPx,
        rootBounds.bottom - imeBottomPx - settledActionBounds.bottom,
        absoluteTolerance = PositionTolerancePx,
      )
      assertEquals(initialPillBounds, settledPillBounds)

      runOnIdle { DesktopDebugKeyboard.hideKeyboardSurface() }
      mainClock.advanceTimeBy(300)
      waitForIdle()

      val returnedActionBounds = onNode(hasClickAction()).fetchSemanticsNode().boundsInRoot
      assertEquals(
        initialActionBounds.bottom,
        returnedActionBounds.bottom,
        absoluteTolerance = PositionTolerancePx,
      )
    } finally {
      mainClock.autoAdvance = true
      runOnIdle {
        DesktopDebugKeyboard.notifyFocusChanged(keyboardOwner, isFocused = false)
        DesktopDebugKeyboard.updateHardwareKeyboardConnected(previousHardwareKeyboardConnected)
        DesktopDebugKeyboard.hideKeyboardSurface()
      }
    }
  }

  @Test
  fun actionMenuKeepsItsOffsetFromTheImeAwareAnchor() = runComposeUiTest {
    val keyboardOwner = Any()
    val previousHardwareKeyboardConnected = DesktopDebugKeyboard.hardwareKeyboardConnected
    var navigationBarsBottomPx = 0f
    var imeBottomPx = 0f
    var bottomPaddingPx = 0f
    var imeGapPx = 0f
    val menuEnabled = mutableStateOf(false)

    try {
      setContent {
        BottomBarTestTheme {
          val density = LocalDensity.current
          val navigationBars = WindowInsets.navigationBars
          val ime = WindowInsets.ime
          val menus = remember { listOf(ActionMenuItem(icon = Lucide.Plus, label = MenuLabel)) }
          SideEffect {
            navigationBarsBottomPx = navigationBars.getBottom(density).toFloat()
            imeBottomPx = ime.getBottom(density).toFloat()
            bottomPaddingPx = with(density) { BottomBarDefaults.BottomPadding.toPx() }
            imeGapPx = with(density) { ImeGap.toPx() }
          }

          Box(Modifier.testTag(MenuRootTag).size(width = 400.dp, height = 700.dp)) {
            BottomBarActionButton(
              icon = Lucide.Plus,
              menus = if (menuEnabled.value) menus else emptyList(),
            )
          }
        }
      }

      runOnIdle {
        DesktopDebugKeyboard.updateHardwareKeyboardConnected(false)
        DesktopDebugKeyboard.hideKeyboardSurface()
      }
      mainClock.advanceTimeBy(300)
      waitForIdle()

      val rootBounds = onNodeWithTag(MenuRootTag).fetchSemanticsNode().boundsInRoot
      val hiddenEffectiveBottom =
        maxOf(navigationBarsBottomPx + bottomPaddingPx, imeBottomPx + imeGapPx)
      val hiddenActionCenter = onNode(hasClickAction()).fetchSemanticsNode().boundsInRoot.center
      runOnIdle { menuEnabled.value = true }
      waitForIdle()
      openActionMenu(MenuRootTag, hiddenActionCenter - rootBounds.topLeft)
      waitForIdle()
      val hiddenMenuBounds = onNodeWithText(MenuLabel).fetchSemanticsNode().boundsInRoot

      onNodeWithTag(MenuRootTag).performTouchInput {
        down(Offset(12f, 12f))
        up()
      }
      waitForIdle()
      runOnIdle { menuEnabled.value = false }
      waitForIdle()

      runOnIdle { DesktopDebugKeyboard.notifyFocusChanged(keyboardOwner, isFocused = true) }
      waitForIdle()
      val shownEffectiveBottom =
        maxOf(navigationBarsBottomPx + bottomPaddingPx, imeBottomPx + imeGapPx)
      assertTrue(shownEffectiveBottom > hiddenEffectiveBottom)

      val shownActionCenter = onNode(hasClickAction()).fetchSemanticsNode().boundsInRoot.center
      runOnIdle { menuEnabled.value = true }
      waitForIdle()
      openActionMenu(MenuRootTag, shownActionCenter - rootBounds.topLeft)
      waitForIdle()
      val shownMenuBounds = onNodeWithText(MenuLabel).fetchSemanticsNode().boundsInRoot

      assertEquals(
        shownEffectiveBottom - hiddenEffectiveBottom,
        hiddenMenuBounds.top - shownMenuBounds.top,
        absoluteTolerance = PositionTolerancePx,
      )
    } finally {
      runOnIdle {
        DesktopDebugKeyboard.notifyFocusChanged(keyboardOwner, isFocused = false)
        DesktopDebugKeyboard.updateHardwareKeyboardConnected(previousHardwareKeyboardConnected)
        DesktopDebugKeyboard.hideKeyboardSurface()
      }
    }
  }

  private fun ComposeUiTest.openActionMenu(rootTag: String, actionCenter: Offset) {
    onNodeWithTag(rootTag).performTouchInput {
      down(actionCenter)
      up()
    }
  }

  @Composable
  private fun BottomBarTestTheme(content: @Composable () -> Unit) {
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
    const val RootTag = "bottom-bar-root"
    const val MenuRootTag = "bottom-bar-menu-root"
    const val PillTag = "bottom-bar-pill"
    const val MenuLabel = "Test action"
    val PillKey = Any()
    val ActionKey = Any()
    val ImeGap = 12.dp
    const val PositionTolerancePx = 0.5f
  }
}
