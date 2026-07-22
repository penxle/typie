package co.typie.screen.editor.editor

import androidx.compose.foundation.focusable
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.size
import androidx.compose.runtime.Composable
import androidx.compose.runtime.CompositionLocalProvider
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.SideEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.setValue
import androidx.compose.runtime.withFrameNanos
import androidx.compose.ui.Modifier
import androidx.compose.ui.focus.FocusRequester
import androidx.compose.ui.focus.focusRequester
import androidx.compose.ui.focus.onFocusChanged
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.test.ComposeUiTest
import androidx.compose.ui.test.ExperimentalTestApi
import androidx.compose.ui.test.assertIsFocused
import androidx.compose.ui.test.assertIsNotFocused
import androidx.compose.ui.test.onNodeWithTag
import androidx.compose.ui.test.performTouchInput
import androidx.compose.ui.test.runComposeUiTest
import androidx.compose.ui.unit.dp
import co.typie.editor.Editor
import co.typie.editor.FakeFfiEditor
import co.typie.editor.ffi.Affinity
import co.typie.editor.ffi.ChainSegment
import co.typie.editor.ffi.Position
import co.typie.editor.ffi.Selection
import co.typie.editor.ffi.StablePosition
import co.typie.editor.ffi.StableSelection
import co.typie.ext.clickable
import co.typie.ui.component.dialog.Dialog
import co.typie.ui.component.dialog.DialogOverlay
import co.typie.ui.component.dialog.DialogScope
import co.typie.ui.theme.LightAppShadows
import co.typie.ui.theme.LightColors
import co.typie.ui.theme.LocalAppColors
import co.typie.ui.theme.LocalAppShadows
import co.typie.ui.theme.LocalThemeMode
import co.typie.ui.theme.ResolvedThemeMode
import dev.chrisbanes.haze.blur.HazeBlurStyle
import dev.chrisbanes.haze.blur.LocalHazeBlurStyle
import kotlin.test.Test
import kotlin.test.assertNotNull
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.Job

@OptIn(ExperimentalTestApi::class)
class EditorCommonOverlayFocusReturnDesktopTest {
  @Test
  fun focusedEditorReturnsBeforeDialogRemovalAndRemainsFocused() = runComposeUiTest {
    val fixture = FocusReturnFixture()
    setContent { FocusReturnContent(fixture = fixture, initiallyFocused = true) }

    dismissDialog(fixture)
    onNodeWithTag(EditorFocusTag).assertIsFocused()

    mainClock.advanceTimeBy(250)
    waitForIdle()
    onNodeWithTag(EditorFocusTag).assertIsFocused()
  }

  @Test
  fun dialogOpenedFromUnfocusedEditorDoesNotFocusItOnDismissal() = runComposeUiTest {
    val fixture = FocusReturnFixture()
    setContent { FocusReturnContent(fixture = fixture, initiallyFocused = false) }

    dismissDialog(fixture)
    onNodeWithTag(EditorFocusTag).assertIsNotFocused()
  }

  private fun ComposeUiTest.dismissDialog(fixture: FocusReturnFixture) {
    waitUntil(timeoutMillis = 5_000) { fixture.dialog.current != null }
    waitForIdle()

    mainClock.autoAdvance = false
    onNodeWithTag(DialogDismissTag).performTouchInput {
      down(center)
      up()
    }
    repeat(2) { mainClock.advanceTimeByFrame() }
    waitForIdle()

    assertNotNull(fixture.dialog.current)
  }
}

private class FocusReturnFixture {
  val dialog = Dialog()
  private val selection = selection("selected")
  val editor =
    Editor(
      inner = FakeFfiEditor(selectionProvider = { selection }),
      scope = CoroutineScope(Job()),
      dispatcher = Dispatchers.Unconfined,
    )

  init {
    editor.sync {}
  }
}

@Composable
private fun FocusReturnContent(fixture: FocusReturnFixture, initiallyFocused: Boolean) {
  FocusReturnTestTheme {
    val editorFocusRequester = remember { FocusRequester() }
    val dialogFocusRequester = remember { FocusRequester() }
    val scope = rememberCoroutineScope()
    val session =
      remember(fixture.editor) {
        EditorFocusReturnSession(
          scope = scope,
          freezeSelection = { _, selection -> stableSelection(selection.anchor.node) },
          applySelection = { _, _ -> },
          focusEditor = { editorFocusRequester.requestFocus() },
          awaitFocusBoundary = { withFrameNanos {} },
        )
      }
    var editorFocused by remember { mutableStateOf(false) }

    Box(Modifier.size(400.dp)) {
      Column {
        Box(
          Modifier.testTag(EditorFocusTag)
            .size(48.dp)
            .focusRequester(editorFocusRequester)
            .onFocusChanged { editorFocused = it.isFocused }
            .focusable()
        )
      }

      SideEffect {
        session.observeEditorContext(
          editor = fixture.editor,
          focused = editorFocused,
          selection = fixture.editor.state.selection,
          contextActive = true,
          auxiliaryOwnerActive = fixture.dialog.acceptsInput,
        )
      }
      LaunchedEffect(fixture.dialog.acceptsInput) {
        if (!fixture.dialog.acceptsInput) {
          session.restore()
        }
      }
      LaunchedEffect(Unit) {
        if (initiallyFocused) {
          editorFocusRequester.requestFocus()
          withFrameNanos {}
        }
        fixture.dialog.present<Unit> { FocusReturnDialogContent(dialogFocusRequester) }
      }

      DialogOverlay(fixture.dialog)
    }
  }
}

@Composable
context(scope: DialogScope<Unit>)
private fun FocusReturnDialogContent(focusRequester: FocusRequester) {
  Column(Modifier.size(200.dp)) {
    Box(Modifier.size(48.dp).focusRequester(focusRequester).focusable())
    Box(Modifier.testTag(DialogDismissTag).size(48.dp).clickable { scope.dismiss() })
  }
  LaunchedEffect(Unit) { focusRequester.requestFocus() }
}

@Composable
private fun FocusReturnTestTheme(content: @Composable () -> Unit) {
  CompositionLocalProvider(
    LocalAppColors provides LightColors,
    LocalAppShadows provides LightAppShadows,
    LocalThemeMode provides ResolvedThemeMode.Light,
    LocalHazeBlurStyle provides
      HazeBlurStyle(blurRadius = 20.dp, noiseFactor = 0f, colorEffects = listOf()),
    content = content,
  )
}

private fun selection(node: String): Selection {
  val position = Position(node = node, offset = 0, affinity = Affinity.Downstream)
  return Selection(anchor = position, head = position)
}

private fun stableSelection(node: String): StableSelection {
  val position =
    StablePosition(
      chain = listOf(ChainSegment.Real(node)),
      child = null,
      affinity = Affinity.Downstream,
    )
  return StableSelection(version = 2, anchor = position, head = position)
}

private const val EditorFocusTag = "editor-focus"
private const val DialogDismissTag = "dialog-dismiss"
