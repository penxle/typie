package co.typie.screen.editor.editor.findreplace

import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.size
import androidx.compose.runtime.CompositionLocalProvider
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.test.ExperimentalTestApi
import androidx.compose.ui.test.assertTextEquals
import androidx.compose.ui.test.click
import androidx.compose.ui.test.hasSetTextAction
import androidx.compose.ui.test.onNodeWithText
import androidx.compose.ui.test.performTouchInput
import androidx.compose.ui.test.v2.runComposeUiTest
import androidx.compose.ui.unit.dp
import co.typie.dev.DesktopDebugKeyboard
import co.typie.dev.ProvideDesktopDebugKeyboardPresentation
import co.typie.ext.rememberTextInputState
import co.typie.navigation.LocalRoute
import co.typie.route.Route
import co.typie.ui.component.topbar.LocalTopBarState
import co.typie.ui.component.topbar.ProvideTopBar
import co.typie.ui.component.topbar.TopBar
import co.typie.ui.component.topbar.TopBarCenterAppearance
import co.typie.ui.component.topbar.TopBarState
import co.typie.ui.theme.LightAppShadows
import co.typie.ui.theme.LightColors
import co.typie.ui.theme.LocalAppColors
import co.typie.ui.theme.LocalAppShadows
import co.typie.ui.theme.LocalThemeMode
import co.typie.ui.theme.ResolvedThemeMode
import kotlin.test.Test

@OptIn(ExperimentalTestApi::class)
class FindReplaceTopBarDesktopTest {
  @Test
  fun `korean composition survives find session snapshot updates`() = runComposeUiTest {
    val previousHardwareKeyboardConnected = DesktopDebugKeyboard.hardwareKeyboardConnected
    var findText by mutableStateOf("")
    var pendingFindText: String? = null

    try {
      runOnIdle {
        DesktopDebugKeyboard.updateHardwareKeyboardConnected(false)
        DesktopDebugKeyboard.hideKeyboardSurface()
      }
      setContent {
        val topBarState = remember { TopBarState() }
        val session =
          EditorFindReplaceSession(
            active = true,
            findText = findText,
            replaceText = "",
            matchWholeWord = false,
            matchCount = 0,
            activeMatchNumber = null,
            searchInputFocusRequest = 0,
            onOpen = {},
            close = {},
            updateFindText = { pendingFindText = it },
            updateReplaceText = {},
            updateMatchWholeWord = {},
            findPrevious = {},
            findNext = {},
            replace = {},
            replaceAll = {},
          )
        val inputState =
          rememberTextInputState(
            value = session.findText,
            onValueChange = session.updateFindText,
            onDismiss = {},
          )

        CompositionLocalProvider(
          LocalAppColors provides LightColors,
          LocalAppShadows provides LightAppShadows,
          LocalThemeMode provides ResolvedThemeMode.Light,
          LocalRoute provides Route.Editor("document-id"),
          LocalTopBarState provides topBarState,
        ) {
          ProvideDesktopDebugKeyboardPresentation {
            Box(Modifier.size(width = 400.dp, height = 700.dp)) {
              ProvideTopBar(
                leading = null,
                center = { FindReplaceTopBarCenter(session = session, inputState = inputState) },
                centerKey = FindInputKey,
                centerAppearance = TopBarCenterAppearance.ThemeSurface,
                trailing = null,
                scrollOffset = null,
              )
              TopBar(state = topBarState)
              DesktopDebugKeyboard.Overlay(Modifier.fillMaxSize())
            }
          }
        }
      }
      waitForIdle()

      onNodeWithText("kr 뀨♡", useUnmergedTree = true).performTouchInput { click() }
      waitForIdle()
      runOnIdle {
        findText = checkNotNull(pendingFindText)
        pendingFindText = null
      }
      waitForIdle()
      onNodeWithText("kr 뀨♡", useUnmergedTree = true).performTouchInput { click() }
      waitForIdle()

      onNode(hasSetTextAction(), useUnmergedTree = true).assertTextEquals("아")
    } finally {
      runOnIdle {
        DesktopDebugKeyboard.updateHardwareKeyboardConnected(previousHardwareKeyboardConnected)
        DesktopDebugKeyboard.hideKeyboardSurface()
      }
    }
  }

  private companion object {
    val FindInputKey = Any()
  }
}
