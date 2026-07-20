package co.typie.screen.editor.editor

import androidx.compose.foundation.focusable
import androidx.compose.foundation.layout.Box
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.SideEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.runtime.withFrameNanos
import androidx.compose.ui.Modifier
import androidx.compose.ui.focus.FocusRequester
import androidx.compose.ui.focus.focusRequester
import androidx.compose.ui.focus.onFocusChanged
import androidx.compose.ui.test.ExperimentalTestApi
import androidx.compose.ui.test.hasSetTextAction
import androidx.compose.ui.test.performClick
import androidx.compose.ui.test.v2.runComposeUiTest
import co.typie.editor.runtime.EditorUiState
import co.typie.screen.editor.editor.subpane.comments.CommentComposer
import kotlin.test.Test
import kotlin.test.assertTrue

@OptIn(ExperimentalTestApi::class)
class EditorAuxiliaryFocusBoundaryDesktopTest {
  @Test
  fun commentComposerFocusArrivesBeforeTheEditorBlurBoundaryExpires() = runComposeUiTest {
    val fixture = FocusBoundaryFixture()
    mainClock.autoAdvance = false

    setContent { FocusBoundaryContent(fixture) }
    advanceUntil(fixture, "observed:true")

    onNode(hasSetTextAction()).performClick()
    advanceUntil(fixture, BlurBoundaryExpired)

    fixture.assertAuxiliaryFocusPrecedesBlurBoundary()
  }

  private fun androidx.compose.ui.test.ComposeUiTest.advanceUntil(
    fixture: FocusBoundaryFixture,
    event: String,
  ) {
    repeat(MaxFrames) {
      if (event in fixture.events) return
      mainClock.advanceTimeByFrame()
    }
    assertTrue(event in fixture.events, "Expected $event in ${fixture.events}")
  }
}

private class FocusBoundaryFixture {
  val editorFocusRequester = FocusRequester()
  val events = mutableListOf<String>()

  fun assertAuxiliaryFocusPrecedesBlurBoundary() {
    val auxiliaryFocusIndex = events.indexOf("auxiliary:true")
    val blurBoundaryIndex = events.indexOf(BlurBoundaryExpired)
    assertTrue(auxiliaryFocusIndex >= 0, "Missing auxiliary focus event: $events")
    assertTrue(blurBoundaryIndex >= 0, "Missing blur boundary event: $events")
    assertTrue(
      auxiliaryFocusIndex < blurBoundaryIndex,
      "Auxiliary focus must arrive before the editor blur boundary expires: $events",
    )
  }
}

@Composable
private fun FocusBoundaryContent(fixture: FocusBoundaryFixture) {
  val uiState = remember { EditorUiState() }
  var observedEditorFocusOnce by remember { mutableStateOf(false) }

  Box(
    Modifier.focusRequester(fixture.editorFocusRequester)
      .onFocusChanged { state ->
        fixture.events += "editor:${state.isFocused}"
        uiState.updateFocus(state.isFocused)
      }
      .focusable()
  )

  SideEffect {
    fixture.events += "observed:${uiState.focused}"
    if (uiState.focused) {
      observedEditorFocusOnce = true
    }
  }

  LaunchedEffect(uiState.focused, observedEditorFocusOnce) {
    if (observedEditorFocusOnce && !uiState.focused) {
      withFrameNanos {}
      fixture.events += BlurBoundaryExpired
    }
  }

  CommentComposer(
    value = "",
    onValueChange = {},
    placeholder = "코멘트 작성...",
    submitting = false,
    onFocusChange = { focused -> fixture.events += "auxiliary:$focused" },
    onSubmit = {},
  )

  LaunchedEffect(Unit) { fixture.editorFocusRequester.requestFocus() }
}

private const val BlurBoundaryExpired = "blur-boundary-expired"
private const val MaxFrames = 10
