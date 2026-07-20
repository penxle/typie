package co.typie.screen.editor.editor

import co.typie.editor.Editor
import co.typie.editor.FakeFfiEditor
import co.typie.editor.ffi.Affinity
import co.typie.editor.ffi.ChainSegment
import co.typie.editor.ffi.Position
import co.typie.editor.ffi.Selection
import co.typie.editor.ffi.StablePosition
import co.typie.editor.ffi.StableSelection
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertFalse
import kotlin.test.assertTrue
import kotlinx.coroutines.CompletableDeferred
import kotlinx.coroutines.ExperimentalCoroutinesApi
import kotlinx.coroutines.cancelAndJoin
import kotlinx.coroutines.launch
import kotlinx.coroutines.test.StandardTestDispatcher
import kotlinx.coroutines.test.TestScope
import kotlinx.coroutines.test.runCurrent
import kotlinx.coroutines.test.runTest

@OptIn(ExperimentalCoroutinesApi::class)
class EditorFocusReturnSessionTest {
  @Test
  fun capturesOnlyWhenFocusedEditorSelectionHandsFocusToAuxiliaryInput() = runTest {
    val editor = testEditor(selection("eligible"))
    val frozenSelections = mutableListOf<Selection>()
    val session = session(frozenSelections = frozenSelections)

    observe(session, editor, focused = true)
    session.onAuxiliaryInputFocused()
    runCurrent()

    assertEquals(listOf(selection("eligible")), frozenSelections)
  }

  @Test
  fun doesNotCaptureFromUnfocusedOrSelectionlessEditor() = runTest {
    val editor = testEditor(selection("retained"))
    val frozenSelections = mutableListOf<Selection>()
    val session = session(frozenSelections = frozenSelections)

    observe(session, editor, focused = false)
    session.onAuxiliaryInputFocused()
    editor.setSelection(null)
    observe(session, editor, focused = true)
    session.onAuxiliaryInputFocused()
    runCurrent()

    assertTrue(frozenSelections.isEmpty())
  }

  @Test
  fun auxiliaryFocusFreezesTheTransferTimeSelection() = runTest {
    val editor = testEditor(selection("eligible"))
    val frozenSelections = mutableListOf<Selection>()
    val session = session(frozenSelections = frozenSelections)

    observe(session, editor, focused = true)
    editor.setSelection(selection("transfer"))
    session.onAuxiliaryInputFocused()
    runCurrent()

    assertEquals(listOf(selection("transfer")), frozenSelections)
  }

  @Test
  fun editorBlurCanBeClaimedWithinTheFocusBoundary() = runTest {
    val editor = testEditor(selection("eligible"))
    val boundary = CompletableDeferred<Unit>()
    val frozenSelections = mutableListOf<Selection>()
    val session = session(frozenSelections = frozenSelections, awaitFocusBoundary = boundary::await)

    observe(session, editor, focused = true)
    observe(session, editor, focused = false)
    session.onAuxiliaryInputFocused()
    runCurrent()

    assertEquals(listOf(selection("eligible")), frozenSelections)
  }

  @Test
  fun expiredEditorBlurBoundaryCannotBeClaimed() = runTest {
    val editor = testEditor(selection("eligible"))
    val boundary = CompletableDeferred<Unit>()
    val frozenSelections = mutableListOf<Selection>()
    val session = session(frozenSelections = frozenSelections, awaitFocusBoundary = boundary::await)

    observe(session, editor, focused = true)
    observe(session, editor, focused = false)
    boundary.complete(Unit)
    runCurrent()
    session.onAuxiliaryInputFocused()
    runCurrent()

    assertTrue(frozenSelections.isEmpty())
  }

  @Test
  fun additionalAuxiliaryInputFocusKeepsTheOriginalCapture() = runTest {
    val editor = testEditor(selection("eligible"))
    val frozenSelections = mutableListOf<Selection>()
    val session = session(frozenSelections = frozenSelections)

    capture(session, editor)
    session.onAuxiliaryInputFocused()
    runCurrent()

    assertEquals(listOf(selection("eligible")), frozenSelections)
  }

  @Test
  fun currentSelectionWinsWithoutWaitingForOrApplyingCapture() = runTest {
    val editor = testEditor(selection("eligible"))
    val freezeGate = CompletableDeferred<StableSelection?>()
    var applyRequests = 0
    var focusRequests = 0
    val session =
      EditorFocusReturnSession(
        scope = this,
        freezeSelection = { _, _ -> freezeGate.await() },
        applySelection = { _, _ -> applyRequests += 1 },
        focusEditor = { focusRequests += 1 },
        awaitFocusBoundary = {},
      )

    capture(session, editor)
    editor.setSelection(selection("current"))
    session.restore()

    assertEquals(0, applyRequests)
    assertEquals(1, focusRequests)
    assertFalse(freezeGate.isCompleted)
  }

  @Test
  fun selectionAppearingDuringStableResolutionWinsWithoutApplyingCapture() = runTest {
    val editor = testEditor(selection("eligible"))
    val freezeGate = CompletableDeferred<StableSelection?>()
    var applyRequests = 0
    var focusRequests = 0
    val session =
      EditorFocusReturnSession(
        scope = this,
        freezeSelection = { _, _ -> freezeGate.await() },
        applySelection = { _, _ -> applyRequests += 1 },
        focusEditor = { focusRequests += 1 },
        awaitFocusBoundary = {},
      )

    capture(session, editor)
    editor.setSelection(null)
    val restore = launch { session.restore() }
    runCurrent()
    editor.setSelection(selection("current"))
    freezeGate.complete(stableSelection("captured"))
    restore.join()

    assertEquals(0, applyRequests)
    assertEquals(1, focusRequests)
  }

  @Test
  fun capturedSelectionIsAppliedBeforeFocusWhenCurrentSelectionIsMissing() = runTest {
    val editor = testEditor(selection("eligible"))
    val events = mutableListOf<String>()
    val session =
      EditorFocusReturnSession(
        scope = this,
        freezeSelection = { _, _ -> stableSelection("captured") },
        applySelection = { _, stable ->
          events += "apply:${(stable.anchor.chain.single() as ChainSegment.Real).dot}"
          editor.setSelection(selection("restored"))
        },
        focusEditor = { events += "focus" },
        awaitFocusBoundary = {},
      )

    capture(session, editor)
    editor.setSelection(null)
    session.restore()

    assertEquals(listOf("apply:captured", "focus"), events)
  }

  @Test
  fun failedFreezeDoesNotFocusSelectionlessEditor() = runTest {
    val editor = testEditor(selection("eligible"))
    var focusRequests = 0
    val session =
      EditorFocusReturnSession(
        scope = this,
        freezeSelection = { _, _ -> error("freeze failed") },
        applySelection = { _, _ -> error("must not apply") },
        focusEditor = { focusRequests += 1 },
        awaitFocusBoundary = {},
      )

    capture(session, editor)
    editor.setSelection(null)
    session.restore()

    assertEquals(0, focusRequests)
  }

  @Test
  fun failedApplyDoesNotFocusSelectionlessEditor() = runTest {
    val editor = testEditor(selection("eligible"))
    var focusRequests = 0
    val session =
      EditorFocusReturnSession(
        scope = this,
        freezeSelection = { _, _ -> stableSelection("captured") },
        applySelection = { _, _ -> error("apply failed") },
        focusEditor = { focusRequests += 1 },
        awaitFocusBoundary = {},
      )

    capture(session, editor)
    editor.setSelection(null)
    session.restore()

    assertEquals(0, focusRequests)
  }

  @Test
  fun focusFailureIsSilentAndCaptureIsConsumedOnce() = runTest {
    val editor = testEditor(selection("eligible"))
    var focusRequests = 0
    val session =
      EditorFocusReturnSession(
        scope = this,
        freezeSelection = { _, _ -> stableSelection("captured") },
        applySelection = { _, _ -> error("must not apply") },
        focusEditor = {
          focusRequests += 1
          error("focus failed")
        },
        awaitFocusBoundary = {},
      )

    capture(session, editor)
    repeat(2) { session.restore() }

    assertEquals(1, focusRequests)
  }

  @Test
  fun cancellationDuringRestoreBoundaryPreservesCapture() = runTest {
    val editor = testEditor(selection("eligible"))
    val restoreBoundary = CompletableDeferred<Unit>()
    var focusRequests = 0
    val session =
      EditorFocusReturnSession(
        scope = this,
        freezeSelection = { _, _ -> stableSelection("captured") },
        applySelection = { _, _ -> error("must not apply") },
        focusEditor = { focusRequests += 1 },
        awaitFocusBoundary = restoreBoundary::await,
      )

    capture(session, editor)
    val cancelledRestore = launch { session.restore() }
    runCurrent()
    cancelledRestore.cancelAndJoin()
    restoreBoundary.complete(Unit)
    session.restore()

    assertEquals(1, focusRequests)
  }

  @Test
  fun inactiveContextCannotBeRevivedByReobservingTheSameEditor() = runTest {
    val editor = testEditor(selection("eligible"))
    val freezeGate = CompletableDeferred<StableSelection?>()
    var applyRequests = 0
    var focusRequests = 0
    val session =
      EditorFocusReturnSession(
        scope = this,
        freezeSelection = { _, _ -> freezeGate.await() },
        applySelection = { _, _ -> applyRequests += 1 },
        focusEditor = { focusRequests += 1 },
        awaitFocusBoundary = {},
      )

    capture(session, editor)
    editor.setSelection(null)
    val restore = launch { session.restore() }
    runCurrent()
    observe(session, editor, focused = false, contextActive = false)
    observe(session, editor, focused = true)
    freezeGate.complete(stableSelection("captured"))
    restore.join()

    assertEquals(0, applyRequests)
    assertEquals(0, focusRequests)
  }

  @Test
  fun editorReplacementOrRemovalDiscardsCapture() = runTest {
    val editor = testEditor(selection("eligible"))
    val replacement = testEditor(selection("replacement"))
    var focusRequests = 0
    val session = session(focusEditor = { focusRequests += 1 })

    capture(session, editor)
    observe(session, replacement, focused = true)
    session.restore()
    capture(session, replacement)
    session.observeEditorContext(
      editor = null,
      focused = false,
      selection = null,
      contextActive = true,
    )
    session.restore()

    assertEquals(0, focusRequests)
  }

  @Test
  fun explicitInvalidationDiscardsCapture() = runTest {
    val editor = testEditor(selection("eligible"))
    var focusRequests = 0
    val session = session(focusEditor = { focusRequests += 1 })

    capture(session, editor)
    session.invalidate()
    session.restore()

    assertEquals(0, focusRequests)
  }

  private fun TestScope.testEditor(initialSelection: Selection?): TestEditor {
    var selection = initialSelection
    val ffi = FakeFfiEditor(selectionProvider = { selection })
    val editor = Editor(ffi, this, StandardTestDispatcher(testScheduler))
    val testEditor = TestEditor(editor = editor, updateSelection = { selection = it })
    testEditor.setSelection(initialSelection)
    return testEditor
  }

  private fun TestScope.session(
    frozenSelections: MutableList<Selection> = mutableListOf(),
    focusEditor: (Editor) -> Unit = {},
    awaitFocusBoundary: suspend () -> Unit = {},
  ): EditorFocusReturnSession =
    EditorFocusReturnSession(
      scope = this,
      freezeSelection = { _, selection ->
        frozenSelections += selection
        stableSelection(selection.anchor.node)
      },
      applySelection = { _, _ -> },
      focusEditor = focusEditor,
      awaitFocusBoundary = awaitFocusBoundary,
    )

  private fun observe(
    session: EditorFocusReturnSession,
    editor: TestEditor,
    focused: Boolean,
    contextActive: Boolean = true,
  ) {
    session.observeEditorContext(
      editor = editor.editor,
      focused = focused,
      selection = editor.editor.state.selection,
      contextActive = contextActive,
    )
  }

  private fun TestScope.capture(session: EditorFocusReturnSession, editor: TestEditor) {
    observe(session, editor, focused = true)
    session.onAuxiliaryInputFocused()
    runCurrent()
  }
}

private class TestEditor(val editor: Editor, private val updateSelection: (Selection?) -> Unit) {
  fun setSelection(selection: Selection?) {
    updateSelection(selection)
    editor.sync {}
  }
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
