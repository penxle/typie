package co.typie.screen.editor.editor

import androidx.compose.runtime.mutableStateOf
import androidx.compose.ui.test.ExperimentalTestApi
import androidx.compose.ui.test.v2.runComposeUiTest
import co.typie.screen.editor.editor.state.EditorInputEffect
import co.typie.screen.editor.editor.subpane.EditorSubPane
import co.typie.screen.editor.editor.subpane.EditorSubPaneState
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertFalse
import kotlin.test.assertNotNull
import kotlin.test.assertNotSame
import kotlin.test.assertNull
import kotlin.test.assertTrue

@OptIn(ExperimentalTestApi::class)
class EditorScreenReadingModeDesktopTest {
  @Test
  fun openingAuxiliarySubPaneBlursEditorWithoutBlockingLaterInput() {
    val subPaneState = EditorSubPaneState()
    var blurCount = 0

    openEditorAuxiliarySubPane(
      state = subPaneState,
      pane = EditorSubPane.Comments,
      blurEditor = { blurCount += 1 },
    )

    assertEquals(1, blurCount)
    assertEquals(EditorSubPane.Comments, subPaneState.active)
    assertFalse(subPaneState.editorInputBlocked)
    assertTrue(subPaneState.auxiliaryFocusOwnerActive)
  }

  @Test
  fun newEntityStartsInReadingModeAfterThePreviousEntityWasEditing() = runComposeUiTest {
    val entityId = mutableStateOf("A")
    lateinit var state: EditorScreenEditingState

    setContent { state = rememberEditorScreenEditingState(entityId.value) }
    runOnIdle { state.enterEditing() }
    waitForIdle()

    lateinit var previousState: EditorScreenEditingState
    runOnIdle {
      assertTrue(state.editing)
      previousState = state
      entityId.value = "B"
    }
    waitForIdle()

    runOnIdle {
      assertNotSame(previousState, state)
      assertFalse(state.editing)
    }
  }

  @Test
  fun hideKeyboardAndClearFocusDoNotEnterReadingMode() {
    val state = EditorScreenEditingState().apply { enterEditing() }
    var keyboardHidden = false
    var focusCleared = false
    var readingModeRequested = false

    performEditorInputEffects(
      effects = listOf(EditorInputEffect.HideKeyboard, EditorInputEffect.ClearFocus),
      showKeyboard = {},
      hideKeyboard = { keyboardHidden = true },
      requestFocus = {},
      clearFocus = { focusCleared = true },
      enterReadingMode = { readingModeRequested = true },
    )

    assertTrue(keyboardHidden)
    assertTrue(focusCleared)
    assertFalse(readingModeRequested)
    assertTrue(state.editing)
  }

  @Test
  fun explicitInputEffectRequestsTheCoordinatedReadingModeTransition() {
    var readingModeRequested = false

    performEditorInputEffects(
      effects = listOf(EditorInputEffect.EnterReadingMode),
      showKeyboard = {},
      hideKeyboard = {},
      requestFocus = {},
      clearFocus = {},
      enterReadingMode = { readingModeRequested = true },
    )

    assertTrue(readingModeRequested)
  }

  @Test
  fun editingPromotionInvalidatesAnInFlightReadingTransition() {
    val state = EditorScreenEditingState().apply { enterEditing() }
    val transition = assertNotNull(state.beginReadingTransition())

    state.enterEditing()

    assertFalse(state.completeReadingTransition(transition))
    assertTrue(state.editing)
  }

  @Test
  fun readingTransitionDeduplicatesAndCompletesOnlyTheCurrentToken() {
    val state = EditorScreenEditingState().apply { enterEditing() }
    val transition = assertNotNull(state.beginReadingTransition())

    assertNull(state.beginReadingTransition())
    assertTrue(state.isReadingTransitionCurrent(transition))
    assertTrue(state.completeReadingTransition(transition))
    assertFalse(state.editing)
    assertFalse(state.completeReadingTransition(transition))
  }

  @Test
  fun capabilityDowngradeClosesDirectEditingBeforeReadingCleanupRuns() {
    val state = EditorScreenEditingState().apply { enterEditing() }

    assertTrue(state.directEditingEnabled(readOnly = false))
    assertFalse(state.directEditingEnabled(readOnly = true))
    assertTrue(state.editing)

    state.enterReading()
    assertFalse(state.editing)
  }
}
