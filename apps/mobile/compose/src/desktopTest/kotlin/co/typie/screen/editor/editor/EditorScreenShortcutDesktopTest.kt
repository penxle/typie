package co.typie.screen.editor.editor

import androidx.compose.foundation.focusable
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.size
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.focus.FocusManager
import androidx.compose.ui.focus.FocusRequester
import androidx.compose.ui.focus.focusRequester
import androidx.compose.ui.focus.onFocusChanged
import androidx.compose.ui.input.key.Key
import androidx.compose.ui.input.key.KeyEventType
import androidx.compose.ui.input.key.key
import androidx.compose.ui.input.key.onKeyEvent
import androidx.compose.ui.input.key.type
import androidx.compose.ui.platform.LocalFocusManager
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.test.ExperimentalTestApi
import androidx.compose.ui.test.assertIsFocused
import androidx.compose.ui.test.onNodeWithTag
import androidx.compose.ui.test.performKeyInput
import androidx.compose.ui.test.pressKey
import androidx.compose.ui.test.v2.runComposeUiTest
import androidx.compose.ui.unit.dp
import co.typie.ui.input.WindowInputTestHost
import co.typie.ui.utils.platformModUsesMeta
import kotlin.test.Test
import kotlin.test.assertEquals

@OptIn(ExperimentalTestApi::class)
class EditorScreenShortcutDesktopTest {
  @Test
  fun zoomShortcutsAcceptMainKeyboardAndNumpadAndRespectInputBlocking() = runComposeUiTest {
    val focus = FocusRequester()
    val blocked = mutableStateOf(false)
    var zoomIn = 0
    var zoomOut = 0
    var reset = 0
    val actions =
      EditorScreenShortcutActions(
        openFindReplace = {},
        closeFindReplace = {},
        closeSpellcheck = {},
        closeAiFeedback = {},
        zoomIn = { zoomIn++ },
        zoomOut = { zoomOut++ },
        resetZoom = { reset++ },
      )
    setContent {
      WindowInputTestHost(Modifier.size(100.dp)) {
        EditorScreenShortcuts(
          EditorScreenShortcutContext(!blocked.value, true, false, false, false),
          actions,
        )
        Box(Modifier.size(80.dp).testTag("input").focusRequester(focus).focusable())
      }
    }
    runOnIdle { focus.requestFocus() }
    val node = onNodeWithTag("input")
    node.performKeyInput {
      pressKey(Key.Equals)
      pressKey(Key.Minus)
      pressKey(Key.Zero)
      keyDown(if (platformModUsesMeta) Key.MetaLeft else Key.CtrlLeft)
      pressKey(Key.Equals)
      keyDown(Key.ShiftLeft)
      pressKey(Key.Equals)
      keyUp(Key.ShiftLeft)
      pressKey(Key.Plus)
      pressKey(Key.NumPadAdd)
      pressKey(Key.Minus)
      pressKey(Key.NumPadSubtract)
      pressKey(Key.Zero)
      pressKey(Key.NumPad0)
      keyUp(if (platformModUsesMeta) Key.MetaLeft else Key.CtrlLeft)
    }
    assertEquals(4, zoomIn)
    assertEquals(2, zoomOut)
    assertEquals(2, reset)
    runOnIdle { blocked.value = true }
    node.performKeyInput {
      keyDown(if (platformModUsesMeta) Key.MetaLeft else Key.CtrlLeft)
      pressKey(Key.Equals)
      pressKey(Key.Minus)
      pressKey(Key.Zero)
      keyUp(if (platformModUsesMeta) Key.MetaLeft else Key.CtrlLeft)
    }
    assertEquals(4, zoomIn)
    assertEquals(2, zoomOut)
    assertEquals(2, reset)
  }

  @Test
  fun readingZoomRestoresScreenFocusWithoutStealingInputOrModalFocus() = runComposeUiTest {
    var modalOpen by mutableStateOf(false)
    var modeActive by mutableStateOf(true)
    var editorFocused by mutableStateOf(false)
    var zoomIn = 0
    var editorEscapes = 0
    var modeCloses = 0
    var findOpens = 0
    lateinit var focusManager: FocusManager
    val inputFocus = FocusRequester()
    val modalFocus = FocusRequester()
    val actions =
      EditorScreenShortcutActions(
        openFindReplace = { findOpens++ },
        closeFindReplace = { modeCloses++ },
        closeSpellcheck = {},
        closeAiFeedback = {},
        zoomIn = { zoomIn++ },
        zoomOut = {},
        resetZoom = {},
      )
    setContent {
      focusManager = LocalFocusManager.current
      WindowInputTestHost(Modifier.size(200.dp).testTag("window")) {
        EditorScreenShortcuts(
          EditorScreenShortcutContext(!modalOpen, editorFocused, modeActive, false, false),
          actions,
        )
        Box(Modifier.size(150.dp).testTag("screen"))
        // Portaled search/title inputs are siblings of the editor body.
        Box {
          Box(
            Modifier.size(50.dp)
              .testTag("input")
              .focusRequester(inputFocus)
              .onFocusChanged { editorFocused = it.isFocused }
              .onKeyEvent {
                if (it.key == Key.Escape && it.type == KeyEventType.KeyDown) {
                  editorEscapes++
                  true
                } else false
              }
              .focusable()
          )
        }
        if (modalOpen) {
          Box(Modifier.size(100.dp).testTag("modal").focusRequester(modalFocus).focusable())
          LaunchedEffect(Unit) { modalFocus.requestFocus() }
        }
      }
    }
    val screen = onNodeWithTag("window")
    screen.assertIsFocused()
    fun zoom() = screen.performKeyInput {
      keyDown(if (platformModUsesMeta) Key.MetaLeft else Key.CtrlLeft)
      pressKey(Key.Equals)
      keyUp(if (platformModUsesMeta) Key.MetaLeft else Key.CtrlLeft)
    }
    zoom()
    assertEquals(1, zoomIn)
    runOnIdle { inputFocus.requestFocus() }
    onNodeWithTag("input").performKeyInput {
      keyDown(if (platformModUsesMeta) Key.MetaLeft else Key.CtrlLeft)
      pressKey(Key.Equals)
      pressKey(Key.F)
      keyUp(if (platformModUsesMeta) Key.MetaLeft else Key.CtrlLeft)
    }
    assertEquals(2, zoomIn)
    assertEquals(1, findOpens)
    // Mode changes must not move focus away from a portaled input.
    runOnIdle { modeActive = false }
    runOnIdle { modeActive = true }
    onNodeWithTag("input").assertIsFocused().performKeyInput { pressKey(Key.Escape) }
    assertEquals(1, editorEscapes)
    assertEquals(0, modeCloses)
    runOnIdle { focusManager.clearFocus(force = true) }
    screen.assertIsFocused().performKeyInput { pressKey(Key.Escape) }
    assertEquals(1, modeCloses)
    runOnIdle { modeActive = false }
    zoom()
    assertEquals(3, zoomIn)
    runOnIdle { modalOpen = true }
    onNodeWithTag("modal").assertIsFocused()
    zoom()
    assertEquals(3, zoomIn)
    runOnIdle { modalOpen = false }
    screen.assertIsFocused()
    zoom()
    assertEquals(4, zoomIn)
  }
}
